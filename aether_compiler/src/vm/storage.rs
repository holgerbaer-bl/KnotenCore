use crate::executor::RelType;
use crate::vm::machine::VMState;
use serde_json::Value;
use std::fs;
use std::path::Path;

const STORAGE_DIR: &str = ".knoten_data/storage";

pub fn store_value(key: &str, value: &Value) -> Result<(), String> {
    fs::create_dir_all(STORAGE_DIR).map_err(|e| e.to_string())?;
    let path = format!("{}/{}.json", STORAGE_DIR, key);
    let data = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    fs::write(path, data).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_value(key: &str) -> Result<Value, String> {
    let path = format!("{}/{}.json", STORAGE_DIR, key);
    if !Path::new(&path).exists() {
        return Ok(Value::Null);
    }
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

pub fn serialize_vm_state(state: &VMState) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = Vec::new();

    buf.extend_from_slice(&0x4B4E4343u32.to_le_bytes());

    buf.extend_from_slice(&(state.globals.len() as u64).to_le_bytes());
    for (key, val) in &state.globals {
        let key_bytes = key.as_bytes();
        buf.extend_from_slice(&(key_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(key_bytes);
        encode_reltype(val, &mut buf)?;
    }

    buf.extend_from_slice(&(state.stack.len() as u64).to_le_bytes());
    for val in &state.stack {
        encode_reltype(val, &mut buf)?;
    }

    buf.extend_from_slice(&(state.frames.len() as u64).to_le_bytes());
    for frame in &state.frames {
        buf.extend_from_slice(&(frame.ip as u64).to_le_bytes());
        buf.extend_from_slice(&(frame.base_pointer as u64).to_le_bytes());
    }

    buf.extend_from_slice(&(state.ip as u64).to_le_bytes());
    buf.extend_from_slice(&(state.base_pointer as u64).to_le_bytes());
    buf.extend_from_slice(&state.crypto_state_hash.to_le_bytes());
    buf.extend_from_slice(&state.nonce.to_le_bytes());
    buf.extend_from_slice(&state.previous_state_hash);

    Ok(buf)
}

pub fn deserialize_vm_state(bytes: &[u8]) -> Result<VMState, String> {
    if bytes.len() < 12 {
        return Err("Data too short".into());
    }
    let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    if magic != 0x4B4E4343 {
        return Err(format!("Invalid magic: 0x{magic:08X}"));
    }
    let mut pos: usize = 4;

    let globals_count = read_u64(bytes, &mut pos)? as usize;
    let mut globals = std::collections::HashMap::new();
    for _ in 0..globals_count {
        let key = read_string(bytes, &mut pos)?;
        let val = decode_reltype(bytes, &mut pos)?;
        globals.insert(key, val);
    }

    let stack_count = read_u64(bytes, &mut pos)? as usize;
    let mut stack = Vec::with_capacity(stack_count);
    for _ in 0..stack_count {
        stack.push(decode_reltype(bytes, &mut pos)?);
    }

    let frames_count = read_u64(bytes, &mut pos)? as usize;
    let mut frames = Vec::with_capacity(frames_count);
    for _ in 0..frames_count {
        let ip = read_u64(bytes, &mut pos)? as usize;
        let bp = read_u64(bytes, &mut pos)? as usize;
        frames.push(crate::vm::machine::CallFrame {
            ip,
            base_pointer: bp,
        });
    }

    let ip = read_u64(bytes, &mut pos)? as usize;
    let base_pointer = read_u64(bytes, &mut pos)? as usize;
    let crypto_state_hash = read_u64(bytes, &mut pos)?;
    let nonce = read_u64(bytes, &mut pos)?;
    let mut previous_state_hash = [0u8; 32];
    if pos + 32 <= bytes.len() {
        previous_state_hash.copy_from_slice(&bytes[pos..pos + 32]);
    }

    Ok(VMState {
        globals,
        stack,
        frames,
        ip,
        base_pointer,
        crypto_state_hash,
        nonce,
        previous_state_hash,
    })
}

fn encode_reltype(val: &RelType, buf: &mut Vec<u8>) -> Result<(), String> {
    match val {
        RelType::Int(v) => {
            buf.push(0);
            buf.extend_from_slice(&v.to_le_bytes());
        }
        RelType::Float(v) => {
            buf.push(1);
            buf.extend_from_slice(&v.to_le_bytes());
        }
        RelType::Bool(v) => {
            buf.push(if *v { 2 } else { 3 });
        }
        RelType::Str(s) => {
            buf.push(4);
            let b = s.as_bytes();
            buf.extend_from_slice(&(b.len() as u32).to_le_bytes());
            buf.extend_from_slice(b);
        }
        RelType::Void => {
            buf.push(5);
        }
        _ => {
            buf.push(5);
        }
    }
    Ok(())
}

fn decode_reltype(bytes: &[u8], pos: &mut usize) -> Result<RelType, String> {
    if *pos >= bytes.len() {
        return Err("Unexpected end of data".into());
    }
    let tag = bytes[*pos];
    *pos += 1;
    match tag {
        0 => {
            let v = i64::from_le_bytes(read_fixed(bytes, pos)?);
            Ok(RelType::Int(v))
        }
        1 => {
            let v = f64::from_le_bytes(read_fixed(bytes, pos)?);
            Ok(RelType::Float(v))
        }
        2 => Ok(RelType::Bool(true)),
        3 => Ok(RelType::Bool(false)),
        4 => {
            let len = read_u32(bytes, pos)? as usize;
            if *pos + len > bytes.len() {
                return Err("String length exceeds data".into());
            }
            let s = String::from_utf8(bytes[*pos..*pos + len].to_vec())
                .map_err(|e| format!("Invalid UTF-8: {e}"))?;
            *pos += len;
            Ok(RelType::Str(s))
        }
        5 => Ok(RelType::Void),
        _ => Ok(RelType::Void),
    }
}

fn read_u64(bytes: &[u8], pos: &mut usize) -> Result<u64, String> {
    let arr = read_fixed(bytes, pos)?;
    Ok(u64::from_le_bytes(arr))
}

fn read_u32(bytes: &[u8], pos: &mut usize) -> Result<u32, String> {
    let arr = read_fixed(bytes, pos)?;
    Ok(u32::from_le_bytes(arr))
}

fn read_string(bytes: &[u8], pos: &mut usize) -> Result<String, String> {
    if *pos + 2 > bytes.len() {
        return Err("Unexpected end".into());
    }
    let len = u16::from_le_bytes([bytes[*pos], bytes[*pos + 1]]) as usize;
    *pos += 2;
    if *pos + len > bytes.len() {
        return Err("String exceeds data".into());
    }
    let s = String::from_utf8(bytes[*pos..*pos + len].to_vec())
        .map_err(|e| format!("Invalid UTF-8: {e}"))?;
    *pos += len;
    Ok(s)
}

fn read_fixed<const N: usize>(bytes: &[u8], pos: &mut usize) -> Result<[u8; N], String> {
    if *pos + N > bytes.len() {
        return Err("Unexpected end of data".into());
    }
    let mut arr = [0u8; N];
    arr.copy_from_slice(&bytes[*pos..*pos + N]);
    *pos += N;
    Ok(arr)
}

pub fn persist_snapshot_to_disk(slot_id: &str, state: &VMState) -> Result<(), String> {
    fs::create_dir_all(STORAGE_DIR).map_err(|e| e.to_string())?;
    let bytes = serialize_vm_state(state)?;
    let path = format!("{}/{}.snap", STORAGE_DIR, slot_id);
    fs::write(&path, bytes).map_err(|e| e.to_string())
}

pub fn load_snapshot_from_disk(slot_id: &str) -> Option<VMState> {
    let path = format!("{}/{}.snap", STORAGE_DIR, slot_id);
    if !Path::new(&path).exists() {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    let state = deserialize_vm_state(&bytes).ok()?;
    if !super::machine::verify_ledger_hash(&state) {
        eprintln!("[Ledger] Verification failed for snapshot {}", slot_id);
        return None;
    }
    Some(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::RelType;
    use crate::vm::machine;
    use crate::vm::machine::VM;

    #[test]
    fn test_vm_state_disk_serialization() {
        let mut vm = VM::new();
        vm.globals.insert("x".to_string(), RelType::Int(42));
        vm.globals
            .insert("name".to_string(), RelType::Str("test".to_string()));
        vm.stack.push(RelType::Int(10));
        vm.stack.push(RelType::Float(3.15));
        vm.ip = 7;
        vm.base_pointer = 2;
        vm.crypto_state_hash = 0xDEADBEEFCAFEu64;

        let state = vm.snapshot();

        let serialized = serialize_vm_state(&state).expect("Serialization must succeed");
        assert!(!serialized.is_empty());
        assert_eq!(&serialized[0..4], &[0x43, 0x43, 0x4E, 0x4B]);

        let deserialized = deserialize_vm_state(&serialized).expect("Deserialization must succeed");

        assert_eq!(deserialized.globals.len(), state.globals.len());
        assert_eq!(deserialized.globals.get("x"), Some(&RelType::Int(42)));
        assert_eq!(
            deserialized.globals.get("name"),
            Some(&RelType::Str("test".to_string()))
        );
        assert_eq!(deserialized.stack.len(), state.stack.len());
        assert_eq!(deserialized.stack[0], RelType::Int(10));
        assert!(
            (match deserialized.stack[1] {
                RelType::Float(f) => (f - 3.15).abs() < 0.001,
                _ => false,
            })
        );
        assert_eq!(deserialized.ip, state.ip);
        assert_eq!(deserialized.base_pointer, state.base_pointer);
        assert_eq!(deserialized.crypto_state_hash, state.crypto_state_hash);

        persist_snapshot_to_disk("test_slot", &state).expect("Disk persistence must succeed");
        let loaded = load_snapshot_from_disk("test_slot").expect("Disk load must succeed");
        assert_eq!(loaded.globals.get("x"), Some(&RelType::Int(42)));
        assert_eq!(loaded.ip, state.ip);
        assert_eq!(loaded.crypto_state_hash, state.crypto_state_hash);

        let _ = fs::remove_file(format!("{}/test_slot.snap", STORAGE_DIR));
    }

    #[test]
    fn test_cryptographic_ledger_chaining() {
        let mut vm = VM::new();
        vm.crypto_state_hash = 100;

        let snap1 = vm.snapshot();
        let n1 = snap1.nonce;
        assert!(machine::verify_ledger_hash(&snap1));

        vm.crypto_state_hash = 200;
        let snap2 = vm.snapshot();
        assert_eq!(snap2.nonce, n1 + 1, "Nonces must be sequential");
        assert!(machine::verify_ledger_hash(&snap2));
        assert_ne!(snap1.previous_state_hash, snap2.previous_state_hash);

        vm.crypto_state_hash = 300;
        let snap3 = vm.snapshot();
        assert_eq!(snap3.nonce, n1 + 2, "Nonces must be sequential");
        assert!(machine::verify_ledger_hash(&snap3));
        assert_ne!(snap2.previous_state_hash, snap3.previous_state_hash);
        assert_ne!(snap1.previous_state_hash, snap3.previous_state_hash);
    }

    #[test]
    fn test_cryptographic_ledger_replay_defense() {
        let mut vm = VM::new();
        vm.crypto_state_hash = 42;
        let snap = vm.snapshot();
        assert!(machine::verify_ledger_hash(&snap));

        let mut tampered = snap.clone();
        tampered.nonce = 999;
        assert!(
            !machine::verify_ledger_hash(&tampered),
            "Tampered nonce must fail verification"
        );

        tampered = snap.clone();
        tampered.previous_state_hash = [0xFFu8; 32];
        assert!(
            !machine::verify_ledger_hash(&tampered),
            "Tampered hash must fail verification"
        );

        tampered = snap.clone();
        tampered.crypto_state_hash = 999;
        assert!(
            !machine::verify_ledger_hash(&tampered),
            "Tampered crypto state hash must fail verification"
        );

        let mut replay_vm = VM::new();
        replay_vm.crypto_state_hash = 1;
        let _ = replay_vm.snapshot();
        let _ = replay_vm.snapshot();
        let later_nonce = machine::get_ledger_nonce();
        let mut replayed = snap.clone();
        replayed.nonce = later_nonce;
        assert!(
            !machine::verify_ledger_hash(&replayed),
            "Replay with wrong ledger hash must fail"
        );
    }
}
