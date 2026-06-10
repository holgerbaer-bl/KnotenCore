"""KnotenCore Python Runtime Binding (ctypes)

Loads the compiled native library and exposes typed wrappers
for the stable C-ABI symbols exported by src/ffi.rs.
"""

import ctypes
import os
import sys
from typing import Optional, Tuple


def _find_library() -> str:
    """Locate the compiled native library."""
    base = os.path.dirname(os.path.dirname(os.path.dirname(
        os.path.dirname(os.path.abspath(__file__)))))
    if sys.platform == "win32":
        candidate = os.path.join(base, "target", "release", "knoten_core.dll")
        if not os.path.exists(candidate):
            candidate = os.path.join(base, "target", "debug", "knoten_core.dll")
    elif sys.platform == "darwin":
        candidate = os.path.join(base, "target", "release", "libknoten_core.dylib")
        if not os.path.exists(candidate):
            candidate = os.path.join(base, "target", "debug", "libknoten_core.dylib")
    else:
        candidate = os.path.join(base, "target", "release", "libknoten_core.so")
        if not os.path.exists(candidate):
            candidate = os.path.join(base, "target", "debug", "libknoten_core.so")
    return candidate


class KnotenCoreRuntime:
    """Python host runtime wrapping the KnotenCore C-ABI."""

    def __init__(self) -> None:
        lib_path = _find_library()
        self._lib = ctypes.CDLL(lib_path)

        self._lib.knotencore_create_vm.restype = ctypes.c_void_p
        self._lib.knotencore_destroy_vm.argtypes = [ctypes.c_void_p]

        self._lib.knotencore_compile_json.argtypes = [
            ctypes.c_char_p, ctypes.c_size_t,
            ctypes.POINTER(ctypes.c_size_t), ctypes.POINTER(ctypes.c_size_t),
        ]
        self._lib.knotencore_compile_json.restype = ctypes.c_void_p

        self._lib.knotencore_free_code.argtypes = [ctypes.c_void_p]

        self._lib.knotencore_spawn_isolate.argtypes = [
            ctypes.c_void_p, ctypes.c_void_p,
        ]
        self._lib.knotencore_spawn_isolate.restype = ctypes.c_void_p

        self._lib.knotencore_join_isolate.argtypes = [
            ctypes.c_void_p,
            ctypes.POINTER(ctypes.c_int32),
            ctypes.POINTER(ctypes.c_int64),
            ctypes.POINTER(ctypes.c_double),
        ]
        self._lib.knotencore_join_isolate.restype = ctypes.c_void_p

        self._lib.knotencore_free_cstr.argtypes = [ctypes.c_void_p]

    def create_vm(self) -> int:
        """Returns opaque VM pointer."""
        return self._lib.knotencore_create_vm()

    def destroy_vm(self, vm_ptr: int) -> None:
        self._lib.knotencore_destroy_vm(vm_ptr)

    def compile_json(self, json_source: str) -> Tuple[int, int, int]:
        """Returns (code_ptr, instr_len, const_len)."""
        buf = json_source.encode("utf-8")
        instr_len = ctypes.c_size_t(0)
        const_len = ctypes.c_size_t(0)
        ptr = self._lib.knotencore_compile_json(
            buf, len(buf),
            ctypes.byref(instr_len), ctypes.byref(const_len),
        )
        return ptr, instr_len.value, const_len.value

    def spawn_isolate(self, vm_ptr: int, code_ptr: int) -> int:
        """Returns opaque join handle."""
        return self._lib.knotencore_spawn_isolate(vm_ptr, code_ptr)

    def join_isolate(self, handle_ptr: int) -> Tuple[int, int, float, Optional[str]]:
        """Returns (tag, int_val, float_val, err_string_or_none)."""
        tag = ctypes.c_int32(0)
        int_val = ctypes.c_int64(0)
        float_val = ctypes.c_double(0.0)
        err = self._lib.knotencore_join_isolate(handle_ptr, ctypes.byref(tag),
                                                 ctypes.byref(int_val), ctypes.byref(float_val))
        err_str = None
        if err:
            err_str = ctypes.cast(err, ctypes.c_char_p).value.decode("utf-8")
            self._lib.knotencore_free_cstr(err)
        return tag.value, int_val.value, float_val.value, err_str

    def free_code(self, code_ptr: int) -> None:
        self._lib.knotencore_free_code(code_ptr)
