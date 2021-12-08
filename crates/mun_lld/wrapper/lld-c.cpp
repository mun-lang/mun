#include <cstdlib>
#include <lld/Common/Driver.h>
#include <mutex>
#include <iostream>

const char* mun_alloc_str(const std::string& str)
{
  size_t size = str.length();
  if(size > 0)
  {
    char *strPtr = reinterpret_cast<char *>(malloc(size + 1));
    memcpy(strPtr, str.c_str(), size + 1);
    return strPtr;
  }
  return nullptr;
}

// LLD seems not to be thread safe. This is terrible. We basically only allow single threaded access to the driver using
// mutexes. Each type of LLD driver seems to be disconnected so we use a mutex for every type.
std::mutex _coffMutex;
std::mutex _elfMutex;
std::mutex _darwinMutex;
std::mutex _darwinOldMutex;
std::mutex _wasmMutex;

extern "C" {

enum LldFlavor {
  Elf = 0,
  Wasm = 1,
  Darwin = 2,
  DarwinOld = 3,
  Coff = 4,
};

struct LldInvokeResult {
  bool success;
  const char* messages;
};

void mun_link_free_result(LldInvokeResult* result)
{
  if(result->messages)
  {
    free(reinterpret_cast<void *>(const_cast<char*>(result->messages)));
  }
}

LldInvokeResult mun_lld_link(LldFlavor flavor, int argc, const char *const *argv) {
  std::string outputString, errorString;
  llvm::raw_string_ostream outputStream(outputString);
  llvm::raw_string_ostream errorStream(errorString);
  std::vector<const char*> args(argv, argv + argc);
  LldInvokeResult result;
  switch(flavor)
  {
    case Elf:
    {
      args.insert(args.begin(), "lld");               // Issue #1: The first argument MUST be the executable name..
      std::unique_lock<std::mutex> lock(_elfMutex);   // Issue #2: The ELF driver is not thread safe..
      result.success = lld::elf::link(args, false, outputStream, errorStream);
      break;
    }
    case Wasm:
    {
      std::unique_lock<std::mutex> lock(_wasmMutex);
      result.success = lld::wasm::link(args, false, outputStream, errorStream);
      break;
    }
    case Darwin:
    {
      std::unique_lock <std::mutex> lock(_darwinMutex);
      result.success = lld::macho::link(args, false, outputStream, errorStream);
      break;
    }
    case DarwinOld:
    {
      std::unique_lock <std::mutex> lock(_darwinOldMutex);
      result.success = lld::mach_o::link(args, false, outputStream, errorStream);
      break;
    }
    case Coff:
    {
      args.insert(args.begin(), "lld.exe");           // Issue #1: The first argument MUST be the executable name..
      std::unique_lock<std::mutex> lock(_coffMutex);  // Issue #2: The COFF driver is not thread safe..
      result.success = lld::coff::link(args, false, outputStream, errorStream);
      break;
    }
    default:
      result.success = false;
      break;
  }
  std::string resultMessage = errorStream.str() + outputStream.str();
  result.messages = mun_alloc_str(resultMessage);
  return result;
}

}
