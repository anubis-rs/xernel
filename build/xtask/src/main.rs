use anyhow::{bail, Result};
use pico_args::Arguments;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use xshell::cmd;
fn main() -> Result<()> {
    
    let mut args = Arguments::from_env();

    // print help message if requested
    if args.contains(["-h", "--help"]) {
        print!("{}", HELP);
        return Ok(());
    }

    // cd into the root folder of this workspace
    let _cwd = xshell::pushd(root());

    match args.subcommand()?.as_deref() {
        Some("build") => {
            // build the kernel
            build()?;
        }
        Some("run") => {
            // first build the kernel
            build()?;

            // then run the produced binray in QEMU
            run()?;
        }

        Some(cmd) => bail!("Unknown subcommand: '{}'", cmd),
        None => bail!("You must supply a subcommand."),
    }

    Ok(())

}

fn build() -> Result<()> {

    let sh = Shell::new();

    cmd!(sh, "build.sh").run()?;

    Ok(())
}

fn run() {

    let sh = Shell::new();

    cmd!(sh, "qemu-system-x86_64 -bios ./uefi-edk2/OVMF.fd -cdrom xernel.iso --no-reboot -d int -D qemulog.log -s -S").run()?;

}

fn root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path
}
