use anyhow::anyhow;
use mun_runtime::Runtime;
use std::path::PathBuf;

use crate::ExitStatus;

#[derive(clap::Args)]
pub struct Args {
    /// The library to use
    library: PathBuf,

    /// The function entry point to call on startup
    #[clap(default_value_t = String::from("main"))]
    entry: String,
}

/// Starts the runtime with the specified library and invokes function `entry`.
pub fn start(args: Args) -> anyhow::Result<ExitStatus> {
    let builder = Runtime::builder(args.library);

    // Safety: we assume that the passed in library is safe
    let runtime = unsafe { builder.finish() }?;

    let fn_definition = runtime
        .get_function_definition(&args.entry)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Failed to obtain entry point '{}'", &args.entry),
            )
        })?;

    let return_type = &fn_definition.prototype.signature.return_type;
    if return_type.equals::<bool>() {
        let result: bool = runtime
            .invoke(&args.entry, ())
            .map_err(|e| anyhow!("{}", e))?;

        println!("{}", result)
    } else if return_type.equals::<f64>() {
        let result: f64 = runtime
            .invoke(&args.entry, ())
            .map_err(|e| anyhow!("{}", e))?;

        println!("{}", result)
    } else if return_type.equals::<i64>() {
        let result: i64 = runtime
            .invoke(&args.entry, ())
            .map_err(|e| anyhow!("{}", e))?;

        println!("{}", result)
    } else if return_type.equals::<()>() {
        #[allow(clippy::unit_arg)]
        runtime
            .invoke(&args.entry, ())
            .map(|_: ()| ExitStatus::Success)
            .map_err(|e| anyhow!("{}", e))?;
    } else {
        return Err(anyhow!(
            "Only native Mun return types are supported for entry points. Found: {}",
            return_type.name()
        ));
    };
    Ok(ExitStatus::Success)
}
