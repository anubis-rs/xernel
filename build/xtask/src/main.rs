use anyhow::{bail, Result};
use dotenv::dotenv;
use fatfs::{format_volume, FormatVolumeOptions};
use pico_args::Arguments;
use std::io::{Cursor, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::{env, fs, vec};
use xshell::{cmd, Shell};

const HELP: &str = "\
xtask
    The build system for xernel
FLAGS:
    -h, --help      Print this message.
    --release       Build the kernel with optimizations.
    --gdb           Start QEMU with GDB server enabled and waiting for a connection.
    --check         Only checks if the format is correct, without making changes (Can only be used with the fmt or lint subcommand)
    --cpus          Set the number CPU cores (default: 2).
    --ram           Set the amount of RAM in given size (M for Megabyte and G for Gigabyte) (default: 128M).
    --wsl-qemu      If you use wsl but got a X server installed like GWSL you can use this flag to say you want to use the qemu you've got installed with your wsl distro and not on windows (also possible to use a env variable called qemu_in_wsl and setting it to true)
    --kvm           Use KVM for QEMU (default: false).
SUBCOMMANDS:
    build           Build the kernel without running it.
    run             Build and run the kernel using QEMU.
    fmt             Run cargo fmt
    clippy          Run clippy
    lint            Run clippy and cargo fmt
";

fn main() -> Result<()> {
    dotenv().ok();
    let mut args = Arguments::from_env();

    // print help message if requested
    if args.contains(["-h", "--help"]) {
        print!("{}", HELP);
        return Ok(());
    }

    let release = args.contains("--release");
    let gdb = args.contains("--gdb");
    let check = args.contains("--check");

    // cd into the root folder of this workspace
    let sh = Shell::new().unwrap();

    sh.set_var("RUSTFLAGS", "-Cforce-frame-pointers=yes -Z macro-backtrace");

    let _cwd = sh.push_dir(root());

    match args.subcommand()?.as_deref() {
        Some("build") => {
            // build the kernel
            build(&sh, release, args)?;
        }
        Some("run") => {
            // first build the kernel
            build(&sh, release, args.clone())?;

            // then run the produced binray in QEMU
            run(&sh, gdb, args)?;
        }
        Some("lint") => {
            fmt(&sh, check)?;
            clippy(&sh)?;
        }
        Some("fmt") => {
            fmt(&sh, check)?;
        }
        Some("clippy") => {
            clippy(&sh)?;
        }

        Some("help") => {
            print!("{}", HELP);
        }

        Some(cmd) => bail!("Unknown subcommand: '{}'", cmd),
        None => bail!("You must supply a subcommand."),
    }

    Ok(())
}

fn build(sh: &Shell, rl: bool, mut args: Arguments) -> Result<()> {
    let target = args
        .opt_value_from_str::<_, String>("--target")?
        .unwrap_or_else(|| "x86_64".to_string());

    if !Path::new(sh.current_dir().as_path().join("xernel/kernel/limine").as_path()).exists() {
        sh.change_dir(sh.current_dir().as_path().join("xernel/kernel"));
        cmd!(
            sh,
            "git clone https://github.com/limine-bootloader/limine.git 
                    --branch=v6.x-branch-binary
                    --depth=1"
        )
        .run()?;
        sh.change_dir(root());
    }

    let release = if rl { &["--release"] } else { &[][..] };

    cmd!(
        sh,
        "cargo build
                {release...}
                -p xernel
                --target ./build/targets/{target}.json
                -Z build-std=core,alloc,compiler_builtins
                -Z build-std-features=compiler-builtins-mem
             "
    )
    .run()?;

    let build_dir = if rl { "release" } else { "debug" };

    let diskname = "xernel.hdd";
    let disksize = 64 * 1024 * 1024; // 64 MB

    let data_vec = vec![0_u8; disksize];
    let mut disk = Cursor::new(data_vec);

    format_volume(&mut disk, FormatVolumeOptions::new().fat_type(fatfs::FatType::Fat32))?;

    let fs = fatfs::FileSystem::new(&mut disk, fatfs::FsOptions::new())?;
    {
        let root_dir = fs.root_dir();

        copy_to_image(&root_dir, &format!("./target/{target}/{build_dir}/xernel"), "xernel")?;

        copy_to_image(&root_dir, "./logo.bmp", "logo.bmp")?;

        let dir = root_dir.create_dir("EFI")?;
        let dir = dir.create_dir("BOOT")?;

        copy_to_image(&dir, "./xernel/kernel/limine/BOOTX64.EFI", "BOOTX64.EFI")?;
        copy_to_image(&dir, "./xernel/kernel/limine.cfg", "limine.cfg")?;
    }
    fs.unmount()?;

    fs::write(diskname, disk.into_inner())?;

    Ok(())
}

fn run(sh: &Shell, gdb: bool, mut args: Arguments) -> Result<()> {
    let gdb_debug = if gdb { &["-S"] } else { &[][..] };

    let ram = args
        .opt_value_from_str::<_, String>("--ram")?
        .unwrap_or_else(|| "128M".to_string());
    let cpus = args.opt_value_from_str::<_, u32>("--cpus")?.unwrap_or(2).to_string();

    let kvm = if args.contains("--kvm") {
        &["-enable-kvm"]
    } else {
        &[][..]
    };

    let mut file_extension = "";

    let qemu_in_wsl_arg = args.contains("--wsl-qemu");

    let qemu_in_wsl_env = env::var("qemu_in_wsl").unwrap_or("false".to_string()).parse().unwrap();

    let qemu_in_wsl = qemu_in_wsl_arg || qemu_in_wsl_env;

    if wsl::is_wsl() && !qemu_in_wsl {
        file_extension = ".exe";
    }

    cmd!(
        sh,
        "qemu-system-x86_64{file_extension}  
                -bios ./xernel/kernel/uefi-edk2/OVMF.fd 
                -m {ram}
                -smp {cpus}
                -cdrom xernel.hdd 
                --no-reboot 
                --no-shutdown
                -debugcon stdio
                -d int 
                -D qemu.log
                {kvm...}
                -s {gdb_debug...}"
    )
    .run()?;

    Ok(())
}

fn clippy(sh: &Shell) -> Result<()> {
    let _cwd = sh.push_dir(root());

    cmd!(
        sh,
        "cargo clippy
            -p xernel
            --target ./build/targets/x86_64.json
            -Z build-std=core,alloc,compiler_builtins
            -Z build-std-features=compiler-builtins-mem"
    )
    .run()?;

    Ok(())
}

fn fmt(sh: &Shell, check: bool) -> Result<()> {
    let _cwd = sh.push_dir(root());

    let check_arg = if check { &["--", "--check"][..] } else { &[] };
    cmd!(
        sh,
        "cargo fmt
            -p xernel
            {check_arg...}"
    )
    .run()?;

    Ok(())
}

fn root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path
}

fn copy_to_image<T: Seek + Write + Read>(dir: &fatfs::Dir<T>, src_path: &str, dst_path: &str) -> Result<()> {
    let data = fs::read(src_path)?;

    dir.create_file(dst_path)?.write_all(&data)?;

    Ok(())
}
