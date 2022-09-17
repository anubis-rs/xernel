use anyhow::{bail, Result};
use pico_args::Arguments;
use std::path::{Path, PathBuf};
use xshell::{cmd, Shell};

const HELP: &str = "\
xtask
    The build system for xernel
FLAGS:
    -h, --help      Print this message.
    --release       Build the kernel with optimizations.
    --gdb           Start QEMU with GDB server enabled and waiting for a connection.
SUBCOMMANDS:
    build           Build the kernel without running it.
    run             Build and run the kernel using QEMU.
    lint            Run clippy and cargo fmt
";

fn main() -> Result<()> {
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

    let _cwd = sh.push_dir(root());

    match args.subcommand()?.as_deref() {
        Some("build") => {
            // build the kernel
            build(&sh, release, args)?;
        }
        Some("run") => {
            // first build the kernel
            build(&sh, release, args)?;

            // then run the produced binray in QEMU
            run(&sh, gdb)?;
        }
        Some("lint") => {
            lint(&sh, check)?;
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

    if !Path::new(
        sh.current_dir()
            .as_path()
            .join("xernel/kernel/limine")
            .as_path(),
    )
    .exists()
    {
        sh.change_dir(sh.current_dir().as_path().join("xernel/kernel"));
        cmd!(
            sh,
            "git clone https://github.com/limine-bootloader/limine.git 
                    --branch=v3.0-branch-binary 
                    --depth=1"
        )
        .run()?;
        cmd!(sh, "make -C limine").run()?;
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

    let diskname = "xernel.hdd";
    let disksize = 64.to_string();

    let bs_unit = match std::env::consts::OS {
        "macos" => "m",
        _ => "M",
    };

    cmd!(
        sh,
        "dd if=/dev/zero of={diskname} bs=1{bs_unit} count=0 seek={disksize}"
    )
    .run()?;

    cmd!(sh, "mformat -i {diskname} -F").run()?;
    cmd!(
        sh,
        "mcopy -i {diskname} ./target/{target}/debug/xernel ::/xernel"
    )
    .run()?;
    cmd!(
        sh,
        "mcopy -i {diskname} xernel/kernel/limine.cfg ::/limine.cfg"
    )
    .run()?;
    cmd!(sh, "mmd -i {diskname} ::/EFI").run()?;
    cmd!(sh, "mmd -i {diskname} ::/EFI/BOOT").run()?;
    cmd!(
        sh,
        "mcopy -i {diskname} xernel/kernel/limine/BOOTX64.EFI ::/EFI/BOOT"
    )
    .run()?;

    Ok(())
}

fn run(sh: &Shell, gdb: bool) -> Result<()> {
    let gdb_debug = if gdb { &["-s", "-S"] } else { &[][..] };

    let mut file_extension = "";

    if wsl::is_wsl() {
        file_extension = ".exe";
    }

    cmd!(
        sh,
        "qemu-system-x86_64{file_extension}
                -bios ./xernel/kernel/uefi-edk2/OVMF.fd 
                -cdrom xernel.hdd 
                --no-reboot 
                --no-shutdown
                -debugcon stdio
                -d int 
                -D qemu.log
                {gdb_debug...}"
    )
    .run()?;

    Ok(())
}

fn lint(sh: &Shell, check: bool) -> Result<()> {
    let _cwd = sh.push_dir(root());

    cmd!(
        sh,
        "cargo clippy
            -p xernel
            --target ./build/targets/x86_64.json
            -Zbuild-std=core,alloc,compiler_builtins"
    )
    .run()?;

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
