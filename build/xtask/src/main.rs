use anyhow::{bail, Result};
use pico_args::Arguments;
use std::path::{PathBuf, Path};
use xshell::{Shell, cmd};

fn main() -> Result<()> {
    
    let mut args = Arguments::from_env();

    // print help message if requested
    if args.contains(["-h", "--help"]) {
        //print!("{}", HELP);
        return Ok(());
    }

    // cd into the root folder of this workspace
    let sh = Shell::new().unwrap();

    let _cwd = sh.push_dir(root());

    match args.subcommand()?.as_deref() {
        Some("build") => {
            // build the kernel
            build(&sh)?;
        }
        Some("run") => {
            // first build the kernel
            build(&sh)?;

            // then run the produced binray in QEMU
            run(&sh)?;
        }

        Some(cmd) => bail!("Unknown subcommand: '{}'", cmd),
        None => bail!("You must supply a subcommand."),
    }

    Ok(())

}

fn build(sh: &Shell) -> Result<()> {

    if !Path::new(sh.current_dir().as_path().join("xernel/kernel/limine").as_path()).exists() {
        sh.change_dir(sh.current_dir().as_path().join("xernel/kernel"));
        cmd!(sh, "git clone https://github.com/limine-bootloader/limine.git 
                    --branch=v3.0-branch-binary 
                    --depth=1").run()?;
        cmd!(sh, "make -C limine").run()?;
        sh.change_dir(root());
    }

    cmd!(sh, "cargo build
                -p xernel
                --target ./build/targets/x86_64.json
                -Z build-std=core,alloc,compiler_builtins
                -Z build-std-features=compiler-builtins-mem
             ").run()?;


    let diskname = "xernel.hdd";
    let disksize = 64.to_string();
    
    cmd!(sh, "dd if=/dev/zero of={diskname} bs=1M count=0 seek={disksize}").run()?;

    cmd!(sh, "mformat -i {diskname} -F").run()?;
    cmd!(sh, "mcopy -i {diskname} ./target/x86_64/debug/xernel ::/xernel").run()?;
    cmd!(sh, "mcopy -i {diskname} xernel/kernel/limine.cfg ::/limine.cfg").run()?;
    cmd!(sh, "mmd -i {diskname} ::/EFI").run()?;
    cmd!(sh, "mmd -i {diskname} ::/EFI/BOOT").run()?;
    cmd!(sh, "mcopy -i {diskname} xernel/kernel/limine/BOOTX64.EFI ::/EFI/BOOT").run()?;

    Ok(())
}

fn run(sh: &Shell) -> Result<()> {

    cmd!(sh, "qemu-system-x86_64 
                -bios ./xernel/kernel/uefi-edk2/OVMF.fd 
                -cdrom xernel.hdd 
                --no-reboot 
                --no-shutdown
                -d int 
                -D qemu.log").run()?;

    Ok(())

}

fn root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path
}
