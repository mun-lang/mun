# Mun Runtime FFI Bindings

C and C++17 bindings for the Mun Runtime.

## Building

Building requires the Mun library files.
You can specify where those are located by specifying either of the CMake options:

* `mun_binaries_path`: Location of a local directory containing the Mun libraries and executable.
* `mun_library_url`: URL to an archive on the web containing the Mun libraries; e.g. `https://github.com/mun-lang/runtime-ffi/releases/download/v0.1.0/mun-runtime-ffi-v0.1.0.tar.gz`.

## Testing

Testing requires the Mun binary. You can specify where it is by passing it as a CMake option through either:

* `mun_binaries_path`: Location of a local directory containing the Mun libraries and executable.
* `mun_executable_url` URL to an archive on the web containing the Mun executable; e.g. `https://github.com/mun-lang/mun/releases/download/v0.1.0/mun-win64-v0.1.0.zip`.

Once CMake has run, you can run the `MunRuntimeTests` executable to run all tests, or use `CTest`.

## License

The Mun Runtime is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or 
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
 
 at your option.
