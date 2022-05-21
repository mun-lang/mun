use anyhow::anyhow;
use clap::ArgMatches;
use mun_runtime::{HasStaticTypeInfo, ReturnTypeReflection, Runtime};

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

    let return_type = &fn_definition.prototype.signature.return_type;
    if return_type.equals::<bool>() {
        let result: bool = runtime
            .invoke(entry_point, ())
            .map_err(|e| anyhow!("{}", e))?;

        println!("{}", result)
    } else if return_type.equals::<f64>() {
        let result: f64 = runtime
            .invoke(entry_point, ())
            .map_err(|e| anyhow!("{}", e))?;

        println!("{}", result)
    } else if return_type.equals::<i64>() {
        let result: i64 = runtime
            .invoke(entry_point, ())
            .map_err(|e| anyhow!("{}", e))?;

        println!("{}", result)
    } else if return_type.equals:: < () >() {
        #[allow(clippy::unit_arg)]
        runtime
            .invoke(entry_point, ())
            .map(|_: ()| ExitStatus::Success)
            .map_err(|e| anyhow!("{}", e))
    } else {
        return Err(anyhow!(
                "Only native Mun return types are supported for entry points. Found: {}",
                ret_type.name()
            ));
    }
    Ok(ExitStatus::Success)
}

fn runtime(matches: &ArgMatches) -> anyhow::Result<Runtime> {
    Runtime::builder(
        matches.value_of("LIBRARY").unwrap(), // Safe because its a required arg
    )
    .finish()
}
