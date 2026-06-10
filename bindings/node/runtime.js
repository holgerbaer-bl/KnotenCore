/**
 * KnotenCore Node.js Runtime Binding (ffi-napi)
 *
 * Requires: npm install ffi-napi ref-napi ref-struct-napi
 *
 * Exposes typed wrappers for the stable C-ABI symbols.
 */
const path = require("path");

let lib;
try {
  const ffi = require("ffi-napi");
  const ref = require("ref-napi");
  const Struct = require("ref-struct-napi");

  const voidPtr = ref.refType(ref.types.void);
  const sizeTPtr = ref.refType("size_t");
  const int32Ptr = ref.refType("int32");
  const int64Ptr = ref.refType("int64");
  const doublePtr = ref.refType("double");

  function findLibrary() {
    const base = path.resolve(__dirname, "..", "..", "..");
    if (process.platform === "win32") {
      return path.join(base, "target", "release", "knoten_core.dll");
    }
    return path.join(base, "target", "release", "libknoten_core.so");
  }

  lib = ffi.Library(findLibrary(), {
    knotencore_create_vm: [voidPtr, []],
    knotencore_destroy_vm: ["void", [voidPtr]],
    knotencore_compile_json: [voidPtr, ["string", "size_t", sizeTPtr, sizeTPtr]],
    knotencore_free_code: ["void", [voidPtr]],
    knotencore_spawn_isolate: [voidPtr, [voidPtr, voidPtr]],
    knotencore_join_isolate: [voidPtr, [voidPtr, int32Ptr, int64Ptr, doublePtr]],
    knotencore_free_cstr: ["void", [voidPtr]],
  });

  function createVM() {
    return lib.knotencore_create_vm();
  }

  function destroyVM(vmPtr) {
    lib.knotencore_destroy_vm(vmPtr);
  }

  function compileJSON(jsonSource) {
    const instrLen = ref.alloc("size_t", 0);
    const constLen = ref.alloc("size_t", 0);
    const ptr = lib.knotencore_compile_json(jsonSource, jsonSource.length,
                                             instrLen, constLen);
    return { ptr, instrLen: instrLen.deref(), constLen: constLen.deref() };
  }

  function spawnIsolate(vmPtr, codePtr) {
    return lib.knotencore_spawn_isolate(vmPtr, codePtr);
  }

  function joinIsolate(handlePtr) {
    const tag = ref.alloc("int32", 0);
    const intVal = ref.alloc("int64", 0);
    const floatVal = ref.alloc("double", 0);
    const err = lib.knotencore_join_isolate(handlePtr, tag, intVal, floatVal);
    let errStr = null;
    if (!err.isNull()) {
      errStr = err.readCString();
      lib.knotencore_free_cstr(err);
    }
    return { tag: tag.deref(), intVal: intVal.deref(),
             floatVal: floatVal.deref(), error: errStr };
  }

  function freeCode(codePtr) {
    lib.knotencore_free_code(codePtr);
  }

  module.exports = { createVM, destroyVM, compileJSON, spawnIsolate,
                     joinIsolate, freeCode };
} catch (_e) {
  module.exports = { createVM() { throw new Error("ffi-napi not available"); },
                     destroyVM() {}, compileJSON() {}, spawnIsolate() {},
                     joinIsolate() {}, freeCode() {} };
}
