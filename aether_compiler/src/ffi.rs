#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::executor::RelType;
use crate::vm::compiler::Compiler;
use crate::vm::machine::VM;
use knoten_core_types::opcode::OpCode;
use std::os::raw::c_char;

struct CompiledCode {
    instructions: Vec<OpCode>,
    constants: Vec<RelType>,
}

#[unsafe(no_mangle)]
pub extern "C" fn knotencore_create_vm() -> *mut VM {
    Box::into_raw(Box::new(VM::new()))
}

#[unsafe(no_mangle)]
pub extern "C" fn knotencore_destroy_vm(ptr: *mut VM) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ptr);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn knotencore_compile_json(
    json_ptr: *const c_char,
    json_len: usize,
    out_instr_len: *mut usize,
    out_const_len: *mut usize,
) -> *mut u8 {
    if json_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let json_str = unsafe {
        let slice = std::slice::from_raw_parts(json_ptr as *const u8, json_len);
        match std::str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        }
    };

    let node: knoten_core_types::ast::Node = match serde_json::from_str(json_str) {
        Ok(n) => n,
        Err(_) => return std::ptr::null_mut(),
    };

    let mut compiler = Compiler::new();
    compiler.compile_node(&node);

    if !out_instr_len.is_null() {
        unsafe {
            *out_instr_len = compiler.instructions.len();
        }
    }
    if !out_const_len.is_null() {
        unsafe {
            *out_const_len = compiler.constants.len();
        }
    }

    Box::into_raw(Box::new(CompiledCode {
        instructions: compiler.instructions,
        constants: compiler.constants,
    })) as *mut u8
}

#[unsafe(no_mangle)]
pub extern "C" fn knotencore_free_code(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ptr as *mut CompiledCode);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn knotencore_spawn_isolate(
    vm_ptr: *mut VM,
    code_ptr: *mut u8,
) -> *mut std::ffi::c_void {
    if vm_ptr.is_null() || code_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let code = unsafe { Box::from_raw(code_ptr as *mut CompiledCode) };
    let parent_globals = unsafe { &*vm_ptr }.globals.clone();
    let code_ref = std::mem::ManuallyDrop::new(code);

    let mut isolate = crate::vm::isolate::VMIsolate::new(
        code_ref.instructions.clone(),
        code_ref.constants.clone(),
    );
    isolate.local_heap.extend(parent_globals);

    let handle = std::thread::spawn(move || isolate.run());

    Box::into_raw(Box::new(handle)) as *mut std::ffi::c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn knotencore_join_isolate(
    handle_ptr: *mut std::ffi::c_void,
    out_tag: *mut i32,
    out_int_val: *mut i64,
    out_float_val: *mut f64,
) -> *mut i8 {
    if handle_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let handle = unsafe {
        Box::from_raw(handle_ptr as *mut std::thread::JoinHandle<Result<RelType, String>>)
    };

    match handle.join() {
        Ok(Ok(val)) => match val {
            RelType::Int(v) => {
                if !out_tag.is_null() {
                    unsafe {
                        *out_tag = 0;
                    }
                }
                if !out_int_val.is_null() {
                    unsafe {
                        *out_int_val = v;
                    }
                }
                std::ptr::null_mut()
            }
            RelType::Float(v) => {
                if !out_tag.is_null() {
                    unsafe {
                        *out_tag = 1;
                    }
                }
                if !out_float_val.is_null() {
                    unsafe {
                        *out_float_val = v;
                    }
                }
                std::ptr::null_mut()
            }
            RelType::Bool(v) => {
                if !out_tag.is_null() {
                    unsafe {
                        *out_tag = if v { 2 } else { 3 };
                    }
                }
                if !out_int_val.is_null() {
                    unsafe {
                        *out_int_val = if v { 1 } else { 0 };
                    }
                }
                std::ptr::null_mut()
            }
            RelType::Str(s) => {
                if !out_tag.is_null() {
                    unsafe {
                        *out_tag = 4;
                    }
                }
                let c_str = std::ffi::CString::new(s).unwrap_or_default();
                c_str.into_raw()
            }
            _ => {
                if !out_tag.is_null() {
                    unsafe {
                        *out_tag = -1;
                    }
                }
                std::ptr::null_mut()
            }
        },
        Ok(Err(e)) => {
            if !out_tag.is_null() {
                unsafe {
                    *out_tag = -2;
                }
            }
            let c_str = std::ffi::CString::new(e).unwrap_or_default();
            c_str.into_raw()
        }
        Err(_) => {
            if !out_tag.is_null() {
                unsafe {
                    *out_tag = -3;
                }
            }
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn knotencore_free_cstr(ptr: *mut i8) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = std::ffi::CString::from_raw(ptr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_abi_facade_embedding() {
        let json = "{\"Add\": [{\"IntLiteral\": 10}, {\"IntLiteral\": 5}]}";

        let vm_ptr = knotencore_create_vm();
        assert!(!vm_ptr.is_null());

        let mut instr_len: usize = 0;
        let mut const_len: usize = 0;

        let code_ptr = knotencore_compile_json(
            json.as_ptr() as *const c_char,
            json.len(),
            &mut instr_len,
            &mut const_len,
        );
        assert!(!code_ptr.is_null());
        assert!(instr_len > 0);

        let handle_ptr = knotencore_spawn_isolate(vm_ptr, code_ptr);
        assert!(!handle_ptr.is_null());

        let mut tag: i32 = 0;
        let mut int_val: i64 = 0;
        let mut float_val: f64 = 0.0;

        let err_ptr = knotencore_join_isolate(handle_ptr, &mut tag, &mut int_val, &mut float_val);
        assert!(err_ptr.is_null(), "Expected no error string");
        assert_eq!(tag, 0, "Expected Int tag");
        assert_eq!(int_val, 15, "10 + 5 = 15");

        knotencore_destroy_vm(vm_ptr);
    }
}
