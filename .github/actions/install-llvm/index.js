const core = require('@actions/core');
const exec = require('@actions/exec');
const tc = require('@actions/tool-cache');
const path = require('path');

const isWindows = process.platform == "win32"
const isMacOS = process.platform == "darwin"
const isLinux = process.platform == "linux"

export async function execute(cmd) {
    let myOutput = '';
    let myError = '';
    await exec.exec(cmd, [], {
        listeners: {
            stdout: (data) => {
                myOutput += data.toString().trim();
            },
            stderr: (data) => {
                myError += data.toString().trim();
            }
        }
    });

    if (myError) {
        throw new Error(myError);
    }
    return myOutput;
}

(async () => {
    try {
        if (isLinux) {
            const installScript = path.join(__dirname, "../../../../scripts/install-llvm.sh");
            await exec.exec(`sudo ${installScript}`);
        } else if (isMacOS) {
            await exec.exec("brew install llvm@14")
            let llvmPath = await execute("brew --prefix llvm@14");
            core.addPath(`${llvmPath}/bin`)
        } else if (isWindows) {
            const downloadUrl = "https://github.com/mun-lang/llvm-package-windows/releases/download/v14.0.6/llvm-14.0.6-windows-x64-msvc16-mt.7z"
            core.info(`downloading LLVM from '${downloadUrl}'`)
            const downloadLocation = await tc.downloadTool(downloadUrl);

            core.info("Succesfully downloaded LLVM release, extracting...")
            const llvmPath = "C:\\llvm";
            const _7zPath = path.join(__dirname, '..', 'externals', '7zr.exe');
            let attempt = 1;
            while (true) {
                const args = [
                    "x", // extract
                    downloadLocation,
                    `-o${llvmPath}`
                ]
                const exit = await exec.exec(_7zPath, args);
                if (exit === 2 && attempt <= 4) {
                    attempt += 1;
                    console.error(`Error extracting LLVM release, retrying attempt #${attempt} after 1s..`)
                    await new Promise(resolve => setTimeout(resolve, 1000));
                }
                else if (exit !== 0) {
                    throw new Error("Could not extract LLVM and Clang binaries.");
                }
                else {
                    core.info("Succesfully extracted LLVM release")
                    break;
                }
            }

            core.addPath(`${llvmPath}/bin`)
            core.exportVariable('LIBCLANG_PATH', `${llvmPath}/bin`)
        } else {
            core.setFailed(`unsupported platform '${process.platform}'`)
        }
    } catch (error) {
        console.error(error.stack);
        core.setFailed(error.message);
    }
})();
