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
        if(isLinux) {
            await exec.exec("sudo apt install llvm-7 llvm-7-* liblld-7*");
        } else if(isMacOS) {
            await exec.exec("brew install llvm@7")
            let llvmPath = await execute("brew --prefix llvm@7");
            core.addPath(`${llvmPath}/bin`)       
        } else if(isWindows) {
            let llvmCachedPath = tc.find("llvm", "7.1.0", "windows-x64");
            if(!llvmCachedPath) {
                let _7zPath = path.join(__dirname, '..', 'externals', '7zr.exe');
                llvmCachedPath = await tc.downloadTool("https://github.com/mun-lang/llvm-package-windows/releases/download/v7.1.0/llvm-7.1.0-windows-x64-msvc15.7z")
                    .then(downloadPath => tc.extract7z(downloadPath, null, _7zPath))
                    .then(extractPath => tc.cacheDir(extractPath, "llvm", "7.1.0", "windows-x64"));
            }
            core.addPath(`${llvmCachedPath}/bin`)
        } else {
            core.setFailed(`unsupported platform '${process.platform}'`)
        }    
    } catch(error) {
        core.setFailed(error.message);
    }
})();
