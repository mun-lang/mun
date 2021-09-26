# Mun Runtime FFI Bindings

C and C++17 bindings for the Mun Runtime.

## Testing

Testing requires the Mun binary. You can specify where it is by passing it as a CMake option through either:

* `MUN_EXECUTABLE_PATH` Path of the Mun executable on your local disk
* `MUN_EXECUTABLE_URL` URL of the location to download the Mun executable from. For example: `https://github.com/mun-lang/mun/releases/download/v0.1.0/mun-win64-v0.1.0.zip`

Once CMake has run, you can run the `MunRuntimeTests` executable to run all tests, or use `CTest`.

## License

The Mun Runtime is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or 
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
 
 at your option.
