use anyhow::anyhow;
use mun_runtime::{ReturnTypeReflection, Runtime};
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
    let runtime = Runtime::builder(args.library).finish()?;

    let fn_definition = runtime
        .get_function_definition(&args.entry)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Failed to obtain entry point '{}'", &args.entry),
            )
        })?;

    if let Some(ret_type) = fn_definition.prototype.signature.return_type() {
        let type_guid = &ret_type.guid;
        if *type_guid == bool::type_guid() {
            let result: bool = runtime
                .invoke(&args.entry, ())
                .map_err(|e| anyhow!("{}", e))?;

            println!("{}", result)
        } else if *type_guid == f64::type_guid() {
            let result: f64 = runtime
                .invoke(&args.entry, ())
                .map_err(|e| anyhow!("{}", e))?;

            println!("{}", result)
        } else if *type_guid == i64::type_guid() {
            let result: i64 = runtime
                .invoke(&args.entry, ())
                .map_err(|e| anyhow!("{}", e))?;

            println!("{}", result)
        } else {
            return Err(anyhow!(
                "Only native Mun return types are supported for entry points. Found: {}",
                ret_type.name()
            ));
        };
        Ok(ExitStatus::Success)
    } else {
        #[allow(clippy::unit_arg)]
        runtime
            .invoke(&args.entry, ())
            .map(|_: ()| ExitStatus::Success)
            .map_err(|e| anyhow!("{}", e))
    }
}
