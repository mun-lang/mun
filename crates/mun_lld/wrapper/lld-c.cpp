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

// The COFF driver seems not to be thread safe. This is terrible. We basically only allow single threaded access
// to the driver using a mutex.
std::mutex _coffMutex;
std::mutex _elfMutex;

extern "C" {

enum LldFlavor {
  Elf = 0,
  Wasm = 1,
  MachO = 2,
  Coff = 3,
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
  std::string errorString;
  llvm::raw_string_ostream errorStream(errorString);
  std::vector<const char*> args(argv, argv + argc);
  LldInvokeResult result;
  switch(flavor)
  {
    case Elf:
    {
      args.insert(args.begin(), "lld");               // Issue #1: The first argument MUST be the executable name..
      std::unique_lock<std::mutex> lock(_elfMutex);   // Issue #2: The ELF driver is not thread safe..
      result.success = lld::elf::link(args, false, errorStream);
      break;
    }
    case Wasm:
      result.success = lld::wasm::link(args, false, errorStream);
      break;
    case MachO:
      result.success = lld::mach_o::link(args, false, errorStream);
      break;
    case Coff:
    {
      args.insert(args.begin(), "lld.exe");           // Issue #1: The first argument MUST be the executable name..
      std::unique_lock<std::mutex> lock(_coffMutex);  // Issue #2: The COFF driver is not thread safe..
      result.success = lld::coff::link(args, false, errorStream);
      break;
    }
    default:
      result.success = false;
      break;
  }
  result.messages = mun_alloc_str(errorStream.str());
  return result;
}

}
