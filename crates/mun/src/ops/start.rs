use anyhow::anyhow;
use clap::ArgMatches;
use mun_runtime::{ReturnTypeReflection, Runtime};

use crate::ExitStatus;

/// Starts the runtime with the specified library and invokes function `entry`.
pub fn start(matches: &ArgMatches) -> anyhow::Result<ExitStatus> {
    let runtime = runtime(matches)?;

    let entry_point = matches.value_of("entry").unwrap_or("main");
    let fn_definition = runtime
        .get_function_definition(entry_point)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Failed to obtain entry point '{}'", entry_point),
            )
        })?;

    if let Some(ret_type) = fn_definition.prototype.signature.return_type() {
        let type_guid = &ret_type.guid;
        if *type_guid == bool::type_guid() {
            let result: bool = runtime
                .invoke(entry_point, ())
                .map_err(|e| anyhow!("{}", e))?;

            println!("{}", result)
        } else if *type_guid == f64::type_guid() {
            let result: f64 = runtime
                .invoke(entry_point, ())
                .map_err(|e| anyhow!("{}", e))?;

            println!("{}", result)
        } else if *type_guid == i64::type_guid() {
            let result: i64 = runtime
                .invoke(entry_point, ())
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
            .invoke(entry_point, ())
            .map(|_: ()| ExitStatus::Success)
            .map_err(|e| anyhow!("{}", e))
    }
}

fn runtime(matches: &ArgMatches) -> anyhow::Result<Runtime> {
    Runtime::builder(
        matches.value_of("LIBRARY").unwrap(), // Safe because its a required arg
    )
    .finish()
}
