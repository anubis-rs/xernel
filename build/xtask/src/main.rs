use anyhow::{bail, Result};
use pico_args::Arguments;
use std::path::PathBuf;
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

    sh.change_dir(root());

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

    sh.change_dir("xernel/kernel");

    cmd!(sh, "cargo build").run()?;

    if !PathBuf::from(sh.current_dir().join("/limine")).exists() {
        cmd!(sh, "git clone https://github.com/limine-bootloader/limine.git 
                    --branch=v3.0-branch-binary 
                    --depth=1").run()?;
        cmd!(sh, "make -C limine").run()?;
    }

    let diskname = "xernel.hdd";
    let disksize = "64";

    cmd!(sh, "dd if=/dev/zero of={diskname} bs=1M count=0 seek={disksize}").run()?;

    cmd!(sh, "mformat -i {diskname} -F").run()?;
    cmd!(sh, "mcopy -i {diskname} ../../target/x86_64/debug/xernel ::/xernel").run()?;
    cmd!(sh, "mcopy -i {diskname} limine.cfg ::/limine.cfg").run()?;
    cmd!(sh, "mmd -i {diskname} ::/EFI").run()?;
    cmd!(sh, "mmd -i {diskname} ::/EFI/BOOT").run()?;
    cmd!(sh, "mcopy -i {diskname} limine/BOOTX64.EFI ::/EFI/BOOT").run()?;

    Ok(())
}

fn run(sh: &Shell) -> Result<()> {

    cmd!(sh, "qemu-system-x86_64 
                -bios ./uefi-edk2/OVMF.fd 
                -cdrom xernel.iso 
                --no-reboot 
                -d int 
                -D qemu.log").run()?;

    Ok(())

}

fn root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path
}
