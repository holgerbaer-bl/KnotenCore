use crate::executor::{AgentPermissions, ExecResult, RelType};

pub trait BridgeModule: Send {
    fn handle(
        &self,
        module: &str,
        function: &str,
        args: &[RelType],
        permissions: &AgentPermissions,
    ) -> Option<ExecResult>;
}

pub struct CoreBridge;

impl BridgeModule for CoreBridge {
    fn handle(
        &self,
        module: &str,
        function: &str,
        args: &[RelType],
        permissions: &AgentPermissions,
    ) -> Option<ExecResult> {
        if module == "test_lib" {
            match function {
                "calculate_hash" => {
                    if args.len() == 1
                        && let RelType::Str(data) = &args[0]
                    {
                        let result = crate::test_lib::calculate_hash(data.clone());
                        return Some(ExecResult::Value(RelType::Int(result)));
                    }
                    Some(ExecResult::Fault {
                        msg: "calculate_hash expects 1 String argument".to_string(),
                        node: "Native::Bridge::calculate_hash".into(),
                    })
                }
                "greet_user" => {
                    if args.len() == 1
                        && let RelType::Str(name) = &args[0]
                    {
                        let result = crate::test_lib::greet_user(name.clone());
                        return Some(ExecResult::Value(RelType::Str(result)));
                    }
                    Some(ExecResult::Fault {
                        msg: "greet_user expects 1 String argument".to_string(),
                        node: "Native::Bridge::greet_user".into(),
                    })
                }
                "normalize_vector" => {
                    if args.len() == 1
                        && let RelType::Object(map) = &args[0]
                    {
                        let x = if let Some(RelType::Float(v)) = map.get("x") {
                            *v
                        } else {
                            return Some(ExecResult::Fault {
                                msg:
                                    "[FFI Error] normalize_vector missing required float field 'x'"
                                        .to_string(),
                                node: "Native::Bridge::normalize_vector".into(),
                            });
                        };
                        let y = if let Some(RelType::Float(v)) = map.get("y") {
                            *v
                        } else {
                            return Some(ExecResult::Fault {
                                msg:
                                    "[FFI Error] normalize_vector missing required float field 'y'"
                                        .to_string(),
                                node: "Native::Bridge::normalize_vector".into(),
                            });
                        };
                        let z = if let Some(RelType::Float(v)) = map.get("z") {
                            *v
                        } else {
                            return Some(ExecResult::Fault {
                                msg:
                                    "[FFI Error] normalize_vector missing required float field 'z'"
                                        .to_string(),
                                node: "Native::Bridge::normalize_vector".into(),
                            });
                        };

                        let input_vec = crate::test_lib::Vector3 { x, y, z };
                        let out_vec = crate::test_lib::normalize_vector(input_vec);

                        let mut out_map = std::collections::HashMap::new();
                        out_map.insert("x".to_string(), RelType::Float(out_vec.x));
                        out_map.insert("y".to_string(), RelType::Float(out_vec.y));
                        out_map.insert("z".to_string(), RelType::Float(out_vec.z));

                        return Some(ExecResult::Value(RelType::Object(out_map)));
                    }
                    Some(ExecResult::Fault {
                        msg: "normalize_vector expects 1 Vector3 Object argument".to_string(),
                        node: "Native::Bridge::normalize_vector".into(),
                    })
                }
                _ => None,
            }
        } else if module == "json" {
            match function {
                "json_parse" => {
                    if args.len() == 1
                        && let RelType::Str(payload) = &args[0]
                    {
                        match crate::natives::fs::fs_parse_json(payload) {
                            Ok(parsed) => return Some(ExecResult::Value(parsed)),
                            Err(e) => {
                                eprintln!("[KnotenCore JSON] Parse error: {}", e);
                                return Some(ExecResult::Value(RelType::Void));
                            }
                        }
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] json_parse expects 1 String arg (payload)".to_string(),
                        node: "Native::Bridge::json_parse".into(),
                    })
                }
                "json_stringify" => {
                    if args.len() == 1 {
                        let json_val = crate::natives::fs::reltype_to_json_value(&args[0]);
                        return Some(ExecResult::Value(RelType::Str(json_val.to_string())));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] json_stringify expects 1 Object/Array argument".to_string(),
                        node: "Native::Bridge::json_stringify".into(),
                    })
                }
                _ => None,
            }
        } else if module == "time" {
            match function {
                "time_sleep_ms" => {
                    if args.len() == 1
                        && let RelType::Int(ms) = &args[0]
                    {
                        if *ms > 0 {
                            std::thread::sleep(std::time::Duration::from_millis(*ms as u64));
                        }
                        return Some(ExecResult::Value(RelType::Void));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] time_sleep_ms expects 1 Int arg (milliseconds)".to_string(),
                        node: "Native::Bridge::time_sleep_ms".into(),
                    })
                }
                // Sprint 188: Formatted local wall-clock time
                "time_get_string" => {
                    let now = chrono::Local::now();
                    let formatted = now.format("%Y-%m-%d %H:%M:%S").to_string();
                    Some(ExecResult::Value(RelType::Str(formatted)))
                }
                // Sprint 188: UTC epoch timestamp in seconds
                "time_utc_timestamp" => {
                    let now = chrono::Utc::now();
                    let stamp = now.timestamp();
                    Some(ExecResult::Value(RelType::Int(stamp)))
                }
                _ => None,
            }
        } else if module == "net" {
            match function {
                "net_fetch" => {
                    if !permissions.allow_network {
                        return Some(ExecResult::Fault {
                            msg: "Permission Denied: allow_network is false (VM: net_fetch). Use --allow-net flag.".to_string(),
                            node: "Native::Bridge::net_fetch".into()
                        });
                    }
                    if args.len() == 1
                        && let RelType::Str(url) = &args[0]
                    {
                        match ureq::get(url).call() {
                            Ok(response) => match response.into_string() {
                                Ok(body) => return Some(ExecResult::Value(RelType::Str(body))),
                                Err(e) => {
                                    return Some(ExecResult::Fault {
                                        msg: format!(
                                            "Network Error: Failed to read response body: {}",
                                            e
                                        ),
                                        node: "Native::Bridge::net_fetch".into(),
                                    });
                                }
                            },
                            Err(e) => {
                                return Some(ExecResult::Fault {
                                    msg: format!("Network Error: HTTP Request Failed: {}", e),
                                    node: "Native::Bridge::net_fetch".into(),
                                });
                            }
                        }
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] net_fetch expects 1 String arg (url)".to_string(),
                        node: "Native::Bridge::net_fetch".into(),
                    })
                }
                "network_get" => {
                    if !permissions.allow_network {
                        return Some(ExecResult::Fault {
                            msg: "Permission Denied: allow_network is false (VM: network_get). Use --allow-net flag.".to_string(),
                            node: "Native::Bridge::network_get".into()
                        });
                    }
                    if args.len() == 1
                        && let RelType::Str(url) = &args[0]
                    {
                        match ureq::get(url).call() {
                            Ok(response) => match response.into_string() {
                                Ok(body) => return Some(ExecResult::Value(RelType::Str(body))),
                                Err(e) => {
                                    return Some(ExecResult::Fault {
                                        msg: format!(
                                            "Network Error: Failed to read response body: {}",
                                            e
                                        ),
                                        node: "Native::Bridge::network_get".into(),
                                    });
                                }
                            },
                            Err(e) => {
                                return Some(ExecResult::Fault {
                                    msg: format!("Network Error: HTTP Request Failed: {}", e),
                                    node: "Native::Bridge::network_get".into(),
                                });
                            }
                        }
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] network_get expects 1 String arg (url)".to_string(),
                        node: "Native::Bridge::network_get".into(),
                    })
                }
                _ => None,
            }
        } else if module == "ui" {
            match function {
                "ui_init_window" => {
                    if args.len() == 3 {
                        let w = match &args[0] {
                            RelType::Int(v) => *v,
                            _ => {
                                return Some(ExecResult::Fault {
                                    msg: "[FFI] ui_init_window: arg 1 must be Int (width)"
                                        .to_string(),
                                    node: "Native::Bridge::ui_init_window".into(),
                                });
                            }
                        };
                        let h = match &args[1] {
                            RelType::Int(v) => *v,
                            _ => {
                                return Some(ExecResult::Fault {
                                    msg: "[FFI] ui_init_window: arg 2 must be Int (height)"
                                        .to_string(),
                                    node: "Native::Bridge::ui_init_window".into(),
                                });
                            }
                        };
                        let title = match &args[2] {
                            RelType::Str(v) => v.clone(),
                            _ => {
                                return Some(ExecResult::Fault {
                                    msg: "[FFI] ui_init_window: arg 3 must be String (title)"
                                        .to_string(),
                                    node: "Native::Bridge::ui_init_window".into(),
                                });
                            }
                        };
                        let ok = crate::natives::ui::ui_init_window(w, h, title);
                        Some(ExecResult::Value(RelType::Bool(ok)))
                    } else {
                        Some(ExecResult::Fault {
                            msg: "[FFI] ui_init_window expects 3 args (width, height, title)"
                                .to_string(),
                            node: "Native::Bridge::ui_init_window".into(),
                        })
                    }
                }
                "ui_clear" => {
                    if args.len() == 1
                        && let RelType::Int(c) = &args[0]
                    {
                        crate::natives::ui::ui_clear(*c);
                        return Some(ExecResult::Value(RelType::Void));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] ui_clear expects 1 Int arg (color)".to_string(),
                        node: "Native::Bridge::ui_clear".into(),
                    })
                }
                "ui_draw_rect" => {
                    if args.len() == 5 {
                        let x = match &args[0] {
                            RelType::Int(v) => *v,
                            _ => {
                                return Some(ExecResult::Fault {
                                    msg: "[FFI] ui_draw_rect: x must be Int".to_string(),
                                    node: "Native::Bridge::ui_draw_rect".into(),
                                });
                            }
                        };
                        let y = match &args[1] {
                            RelType::Int(v) => *v,
                            _ => {
                                return Some(ExecResult::Fault {
                                    msg: "[FFI] ui_draw_rect: y must be Int".to_string(),
                                    node: "Native::Bridge::ui_draw_rect".into(),
                                });
                            }
                        };
                        let w = match &args[2] {
                            RelType::Int(v) => *v,
                            _ => {
                                return Some(ExecResult::Fault {
                                    msg: "[FFI] ui_draw_rect: w must be Int".to_string(),
                                    node: "Native::Bridge::ui_draw_rect".into(),
                                });
                            }
                        };
                        let h = match &args[3] {
                            RelType::Int(v) => *v,
                            _ => {
                                return Some(ExecResult::Fault {
                                    msg: "[FFI] ui_draw_rect: h must be Int".to_string(),
                                    node: "Native::Bridge::ui_draw_rect".into(),
                                });
                            }
                        };
                        let c = match &args[4] {
                            RelType::Int(v) => *v,
                            _ => {
                                return Some(ExecResult::Fault {
                                    msg: "[FFI] ui_draw_rect: color must be Int".to_string(),
                                    node: "Native::Bridge::ui_draw_rect".into(),
                                });
                            }
                        };
                        crate::natives::ui::ui_draw_rect(x, y, w, h, c);
                        Some(ExecResult::Value(RelType::Void))
                    } else {
                        Some(ExecResult::Fault {
                            msg: "[FFI] ui_draw_rect expects 5 args (x, y, w, h, color)"
                                .to_string(),
                            node: "Native::Bridge::ui_draw_rect".into(),
                        })
                    }
                }
                "ui_draw_text" => {
                    if args.len() == 4 {
                        let x = match &args[0] {
                            RelType::Int(v) => *v,
                            _ => {
                                return Some(ExecResult::Fault {
                                    msg: "[FFI] ui_draw_text: x must be Int".to_string(),
                                    node: "Native::Bridge::ui_draw_text".into(),
                                });
                            }
                        };
                        let y = match &args[1] {
                            RelType::Int(v) => *v,
                            _ => {
                                return Some(ExecResult::Fault {
                                    msg: "[FFI] ui_draw_text: y must be Int".to_string(),
                                    node: "Native::Bridge::ui_draw_text".into(),
                                });
                            }
                        };
                        let text = match &args[2] {
                            RelType::Str(v) => v.clone(),
                            _ => {
                                return Some(ExecResult::Fault {
                                    msg: "[FFI] ui_draw_text: text must be String".to_string(),
                                    node: "Native::Bridge::ui_draw_text".into(),
                                });
                            }
                        };
                        let c = match &args[3] {
                            RelType::Int(v) => *v,
                            _ => {
                                return Some(ExecResult::Fault {
                                    msg: "[FFI] ui_draw_text: color must be Int".to_string(),
                                    node: "Native::Bridge::ui_draw_text".into(),
                                });
                            }
                        };
                        crate::natives::ui::ui_draw_text(x, y, text, c);
                        Some(ExecResult::Value(RelType::Void))
                    } else {
                        Some(ExecResult::Fault {
                            msg: "[FFI] ui_draw_text expects 4 args (x, y, text, color)"
                                .to_string(),
                            node: "Native::Bridge::ui_draw_text".into(),
                        })
                    }
                }
                "ui_present" => {
                    let open = crate::natives::ui::ui_present();
                    Some(ExecResult::Value(RelType::Bool(open)))
                }
                "ui_is_key_down" => {
                    if args.len() == 1
                        && let RelType::Str(key) = &args[0]
                    {
                        let down = crate::natives::ui::ui_is_key_down(key.clone());
                        return Some(ExecResult::Value(RelType::Bool(down)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] ui_is_key_down expects 1 String arg".to_string(),
                        node: "Native::Bridge::ui_is_key_down".into(),
                    })
                }
                "ui_get_key_pressed" => {
                    let key = crate::natives::ui::ui_get_key_pressed();
                    Some(ExecResult::Value(RelType::Str(key)))
                }
                // Sprint 118: Text input state binding
                "ui_text_input_get" => {
                    let val = crate::natives::ui::ui_text_input_get();
                    Some(ExecResult::Value(RelType::Str(val)))
                }
                "ui_text_input_set" => {
                    if args.len() == 1
                        && let RelType::Str(s) = &args[0]
                    {
                        crate::natives::ui::ui_text_input_set(s.clone());
                        return Some(ExecResult::Value(RelType::Void));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] ui_text_input_set expects 1 String arg".to_string(),
                        node: "Native::Bridge::ui_text_input_set".into(),
                    })
                }
                _ => None,
            }
        } else if module == "fs" {
            match function {
                "fs_read_file" => {
                    if !permissions.allow_fs_read {
                        return Some(ExecResult::Fault {
                            msg: "Permission Denied: fs.fs_read_file requires FS_READ".to_string(),
                            node: "Bridge::fs.fs_read_file".into(),
                        });
                    }
                    if args.len() == 1
                        && let RelType::Str(path) = &args[0]
                    {
                        let content = crate::natives::fs::fs_read_file(path.clone());
                        return Some(ExecResult::Value(RelType::Str(content)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] fs_read_file expects 1 String arg (path)".to_string(),
                        node: "Native::Bridge::fs_read_file".into(),
                    })
                }
                "fs_parse_json" => {
                    if args.len() == 1
                        && let RelType::Str(json_str) = &args[0]
                    {
                        match crate::natives::fs::fs_parse_json(json_str) {
                            Ok(parsed) => return Some(ExecResult::Value(parsed)),
                            Err(e) => {
                                return Some(ExecResult::Fault {
                                    msg: format!("JSON Parse Error: {}", e),
                                    node: "Native::Bridge::fs_parse_json".into(),
                                });
                            }
                        }
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] fs_parse_json expects 1 String arg (json)".to_string(),
                        node: "Native::Bridge::fs_parse_json".into(),
                    })
                }
                "obj_has_key" => {
                    if args.len() == 2
                        && let (RelType::Object(map), RelType::Str(key)) = (&args[0], &args[1])
                    {
                        return Some(ExecResult::Value(RelType::Bool(map.contains_key(key))));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] obj_has_key expects (Object, String)".to_string(),
                        node: "Native::Bridge::obj_has_key".into(),
                    })
                }
                "obj_set" => {
                    if args.len() == 3
                        && let (RelType::Object(map), RelType::Str(key)) = (&args[0], &args[1])
                    {
                        let mut new_map = map.clone();
                        new_map.insert(key.clone(), args[2].clone());
                        return Some(ExecResult::Value(RelType::Object(new_map)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] obj_set expects (Object, String, Any)".to_string(),
                        node: "Native::Bridge::obj_set".into(),
                    })
                }
                "obj_get" => {
                    if args.len() == 2
                        && let (RelType::Object(map), RelType::Str(key)) = (&args[0], &args[1])
                    {
                        return Some(ExecResult::Value(
                            map.get(key).cloned().unwrap_or(RelType::Void),
                        ));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] obj_get expects (Object, String)".to_string(),
                        node: "Native::Bridge::obj_get".into(),
                    })
                }
                "array_length" => {
                    if args.len() == 1
                        && let RelType::Array(arr) = &args[0]
                    {
                        return Some(ExecResult::Value(RelType::Int(arr.len() as i64)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] array_length expects 1 Array arg".to_string(),
                        node: "Native::Bridge::array_length".into(),
                    })
                }
                "array_get" => {
                    if args.len() == 2
                        && let (RelType::Array(arr), RelType::Int(idx)) = (&args[0], &args[1])
                    {
                        let i = *idx as usize;
                        if i < arr.len() {
                            return Some(ExecResult::Value(arr[i].clone()));
                        }
                        return Some(ExecResult::Value(RelType::Void));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] array_get expects (Array, Int)".to_string(),
                        node: "Native::Bridge::array_get".into(),
                    })
                }
                "array_push" => {
                    if args.len() == 2
                        && let RelType::Array(arr) = &args[0]
                    {
                        let mut new_arr = arr.clone();
                        new_arr.push(args[1].clone());
                        return Some(ExecResult::Value(RelType::Array(new_arr)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] array_push expects (Array, Any)".to_string(),
                        node: "Native::Bridge::array_push".into(),
                    })
                }
                "array_pop" => {
                    if args.len() == 1
                        && let RelType::Array(arr) = &args[0]
                    {
                        let mut new_arr = arr.clone();
                        new_arr.pop();
                        return Some(ExecResult::Value(RelType::Array(new_arr)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] array_pop expects 1 Array arg".to_string(),
                        node: "Native::Bridge::array_pop".into(),
                    })
                }
                "array_len" => {
                    if args.len() == 1
                        && let RelType::Array(arr) = &args[0]
                    {
                        return Some(ExecResult::Value(RelType::Int(arr.len() as i64)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] array_len expects 1 Array arg".to_string(),
                        node: "Native::Bridge::array_len".into(),
                    })
                }
                // Sprint 183: file_read — sandboxed file read with permission check
                "file_read" => {
                    if !permissions.allow_fs_read {
                        return Some(ExecResult::Fault {
                            msg: "Permission Denied: fs.file_read requires FS_READ".to_string(),
                            node: "Bridge::fs.file_read".into(),
                        });
                    }
                    if args.len() == 1
                        && let RelType::Str(path) = &args[0]
                    {
                        if crate::natives::ffi_safety::validate_string(path, "file_read").is_none()
                        {
                            return Some(ExecResult::Fault {
                                msg: "[FFI] file_read: invalid path string".to_string(),
                                node: "Native::Bridge::file_read".into(),
                            });
                        }
                        let content = crate::natives::fs::fs_read_file(path.clone());
                        return Some(ExecResult::Value(RelType::Str(content)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] file_read expects 1 String arg (path)".to_string(),
                        node: "Native::Bridge::file_read".into(),
                    })
                }
                // Sprint 183: file_write — sandboxed file write with permission check
                "file_write" => {
                    if !permissions.allow_fs_write {
                        return Some(ExecResult::Fault {
                            msg: "Permission Denied: fs.file_write requires FS_WRITE".to_string(),
                            node: "Bridge::fs.file_write".into(),
                        });
                    }
                    if args.len() == 2
                        && let (RelType::Str(path), RelType::Str(content)) = (&args[0], &args[1])
                    {
                        if crate::natives::ffi_safety::validate_string(path, "file_write").is_none()
                        {
                            return Some(ExecResult::Fault {
                                msg: "[FFI] file_write: invalid path string".to_string(),
                                node: "Native::Bridge::file_write".into(),
                            });
                        }
                        let ok = std::fs::write(path, content).is_ok();
                        return Some(ExecResult::Value(RelType::Bool(ok)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] file_write expects (String, String)".to_string(),
                        node: "Native::Bridge::file_write".into(),
                    })
                }
                _ => None,
            }
        } else if module == "registry" {
            match function {
                "registry_create_counter" => {
                    let id = crate::natives::registry::registry_create_counter();
                    Some(ExecResult::Value(RelType::Handle(
                        crate::executor::NativeHandle(id),
                    )))
                }
                "registry_increment" => {
                    if args.len() == 1
                        && let RelType::Handle(crate::executor::NativeHandle(id)) = &args[0]
                    {
                        crate::natives::registry::registry_increment(*id);
                        return Some(ExecResult::Value(RelType::Void));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_increment expects 1 Handle arg".to_string(),
                        node: "Native::Bridge::registry_increment".into(),
                    })
                }
                "registry_get_value" => {
                    if args.len() == 1
                        && let RelType::Handle(crate::executor::NativeHandle(id)) = &args[0]
                    {
                        let val = crate::natives::registry::registry_get_value(*id);
                        return Some(ExecResult::Value(RelType::Int(val)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_get_value expects 1 Handle arg".to_string(),
                        node: "Native::Bridge::registry_get_value".into(),
                    })
                }
                "registry_free" => {
                    if args.len() == 1
                        && let RelType::Handle(crate::executor::NativeHandle(id)) = &args[0]
                    {
                        crate::natives::registry::registry_free(*id);
                        return Some(ExecResult::Value(RelType::Void));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_free expects 1 Handle arg".to_string(),
                        node: "Native::Bridge::registry_free".into(),
                    })
                }
                "registry_retain" => {
                    if args.len() == 1
                        && let RelType::Handle(crate::executor::NativeHandle(id)) = &args[0]
                    {
                        crate::natives::registry::registry_retain(*id);
                        return Some(ExecResult::Value(RelType::Void));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_retain expects 1 Handle arg".to_string(),
                        node: "Native::Bridge::registry_retain".into(),
                    })
                }
                "registry_release" => {
                    if args.len() == 1
                        && let RelType::Handle(crate::executor::NativeHandle(id)) = &args[0]
                    {
                        crate::natives::registry::registry_release(*id);
                        return Some(ExecResult::Value(RelType::Void));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_release expects 1 Handle arg".to_string(),
                        node: "Native::Bridge::registry_release".into(),
                    })
                }
                "registry_create_window" => {
                    if args.len() == 3
                        && let (RelType::Int(w), RelType::Int(h), RelType::Str(title)) =
                            (&args[0], &args[1], &args[2])
                    {
                        let id =
                            crate::natives::registry::registry_create_window(*w, *h, title.clone());
                        return Some(ExecResult::Value(RelType::Handle(
                            crate::executor::NativeHandle(id),
                        )));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_create_window expects (Int, Int, String)".to_string(),
                        node: "Native::Bridge::registry_create_window".into(),
                    })
                }
                "registry_window_update" => {
                    if args.len() == 1
                        && let RelType::Handle(crate::executor::NativeHandle(id)) = &args[0]
                    {
                        let open = crate::natives::registry::registry_window_update(*id);
                        return Some(ExecResult::Value(RelType::Bool(open)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_window_update expects 1 Handle arg".to_string(),
                        node: "Native::Bridge::registry_window_update".into(),
                    })
                }
                "registry_window_close" => {
                    if args.len() == 1
                        && let RelType::Handle(crate::executor::NativeHandle(id)) = &args[0]
                    {
                        crate::natives::registry::registry_window_close(*id);
                        return Some(ExecResult::Value(RelType::Void));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_window_close expects 1 Handle arg".to_string(),
                        node: "Native::Bridge::registry_window_close".into(),
                    })
                }
                "registry_dump" => {
                    let total = crate::natives::registry::registry_dump();
                    Some(ExecResult::Value(RelType::Int(total)))
                }
                "registry_file_create" => {
                    if !permissions.allow_fs_write {
                        return Some(ExecResult::Fault {
                            msg:
                                "Permission Denied: registry.registry_file_create requires FS_WRITE"
                                    .to_string(),
                            node: "Bridge::registry.registry_file_create".into(),
                        });
                    }
                    if args.len() == 1
                        && let RelType::Str(path) = &args[0]
                    {
                        if crate::natives::ffi_safety::validate_string(path, "registry_file_create")
                            .is_none()
                        {
                            return Some(ExecResult::Fault {
                                msg: "[FFI] registry_file_create: invalid path string".to_string(),
                                node: "Native::Bridge::registry_file_create".into(),
                            });
                        }
                        let id = crate::natives::registry::registry_file_create(path.clone());
                        return Some(ExecResult::Value(RelType::Handle(
                            crate::executor::NativeHandle(id),
                        )));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_file_create expects 1 String arg".to_string(),
                        node: "Native::Bridge::registry_file_create".into(),
                    })
                }
                "registry_file_write" => {
                    if !permissions.allow_fs_write {
                        return Some(ExecResult::Fault {
                            msg:
                                "Permission Denied: registry.registry_file_write requires FS_WRITE"
                                    .to_string(),
                            node: "Bridge::registry.registry_file_write".into(),
                        });
                    }
                    if args.len() == 2
                        && let (
                            RelType::Handle(crate::executor::NativeHandle(id)),
                            RelType::Str(content),
                        ) = (&args[0], &args[1])
                    {
                        crate::natives::registry::registry_file_write(*id, content.clone());
                        return Some(ExecResult::Value(RelType::Void));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_file_write expects (Handle, String)".to_string(),
                        node: "Native::Bridge::registry_file_write".into(),
                    })
                }
                "registry_now" => {
                    let id = crate::natives::registry::registry_now();
                    Some(ExecResult::Value(RelType::Handle(
                        crate::executor::NativeHandle(id),
                    )))
                }
                "registry_elapsed_ms" => {
                    if args.len() == 1
                        && let RelType::Handle(crate::executor::NativeHandle(id)) = &args[0]
                    {
                        let ms = crate::natives::registry::registry_elapsed_ms(*id);
                        return Some(ExecResult::Value(RelType::Int(ms)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_elapsed_ms expects 1 Handle arg".to_string(),
                        node: "Native::Bridge::registry_elapsed_ms".into(),
                    })
                }
                "registry_gpu_init" => {
                    let id = crate::natives::registry::registry_gpu_init();
                    Some(ExecResult::Value(RelType::Handle(
                        crate::executor::NativeHandle(id),
                    )))
                }
                "registry_fill_color" => {
                    if args.len() == 4
                        && let (
                            RelType::Handle(crate::executor::NativeHandle(win)),
                            RelType::Int(r),
                            RelType::Int(g),
                            RelType::Int(b),
                        ) = (&args[0], &args[1], &args[2], &args[3])
                    {
                        crate::natives::registry::registry_fill_color(*win, *r, *g, *b);
                        return Some(ExecResult::Value(RelType::Void));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_fill_color expects (Handle, Int, Int, Int)"
                            .to_string(),
                        node: "Native::Bridge::registry_fill_color".into(),
                    })
                }
                "registry_is_key_pressed" => {
                    if args.len() == 1
                        && let RelType::Str(key) = &args[0]
                    {
                        let idx = match key.as_str() {
                            "W" => Some(1),
                            "A" => Some(2),
                            "S" => Some(3),
                            "D" => Some(4),
                            "SPACE" => Some(5),
                            "UP" => Some(6),
                            "DOWN" => Some(7),
                            "LEFT" => Some(8),
                            "RIGHT" => Some(9),
                            _ => None,
                        };
                        let pressed = if let Some(i) = idx {
                            crate::natives::registry::GLOBAL_KEYS[i]
                                .load(std::sync::atomic::Ordering::Relaxed)
                        } else {
                            false
                        };
                        return Some(ExecResult::Value(RelType::Bool(pressed)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_is_key_pressed expects 1 String arg (key_name)"
                            .to_string(),
                        node: "Native::Bridge::registry_is_key_pressed".into(),
                    })
                }
                "registry_is_mouse_down" => {
                    let pressed = crate::natives::registry::registry_is_mouse_down();
                    Some(ExecResult::Value(RelType::Bool(pressed)))
                }
                "registry_get_mouse_ray" => {
                    if args.len() == 1
                        && let RelType::Handle(crate::executor::NativeHandle(win)) = &args[0]
                    {
                        let ray = crate::natives::registry::registry_get_mouse_ray(*win);
                        return Some(ExecResult::Value(RelType::Array(ray)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_get_mouse_ray expects 1 Handle arg".to_string(),
                        node: "Native::Bridge::registry_get_mouse_ray".into(),
                    })
                }
                "registry_texture_load" => {
                    if !permissions.allow_fs_read {
                        return Some(ExecResult::Fault {
                            msg:
                                "Permission Denied: registry.registry_texture_load requires FS_READ"
                                    .to_string(),
                            node: "Bridge::registry.registry_texture_load".into(),
                        });
                    }
                    if args.len() == 1
                        && let RelType::Str(path) = &args[0]
                    {
                        if crate::natives::ffi_safety::validate_string(
                            path,
                            "registry_texture_load",
                        )
                        .is_none()
                        {
                            return Some(ExecResult::Fault {
                                msg: "[FFI] registry_texture_load: invalid path string".to_string(),
                                node: "Native::Bridge::registry_texture_load".into(),
                            });
                        }
                        let id = crate::natives::registry::registry_texture_load(path.clone());
                        return Some(ExecResult::Value(RelType::Handle(
                            crate::executor::NativeHandle(id),
                        )));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_texture_load expects 1 String arg".to_string(),
                        node: "Native::Bridge::registry_texture_load".into(),
                    })
                }
                // ── Sprint 162: Retained-Mode UI event routing ──────────────
                // Poll (and clear) whether a UIButton was clicked this frame.
                // Args: (String label) → Bool
                "registry_ui_poll_button" => {
                    if args.len() == 1
                        && let RelType::Str(label) = &args[0]
                    {
                        let clicked =
                            crate::natives::registry::registry_ui_poll_button(label.clone());
                        return Some(ExecResult::Value(RelType::Bool(clicked)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_ui_poll_button expects (String label)".to_string(),
                        node: "Native::Bridge::registry_ui_poll_button".into(),
                    })
                }
                // Read the current text value of a keyed UITextInput widget.
                // Args: (String key) → String
                "registry_ui_read_text" => {
                    if args.len() == 1
                        && let RelType::Str(key) = &args[0]
                    {
                        let val = crate::natives::registry::registry_ui_read_text(key.clone());
                        return Some(ExecResult::Value(RelType::Str(val)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_ui_read_text expects (String key)".to_string(),
                        node: "Native::Bridge::registry_ui_read_text".into(),
                    })
                }
                "registry_spawn_cube" => {
                    if args.len() == 8 {
                        let get_float = |arg: &RelType| -> Option<f32> {
                            match arg {
                                RelType::Float(f) => Some(*f as f32),
                                RelType::Int(i) => Some(*i as f32),
                                _ => None,
                            }
                        };
                        // tex may be a proper Handle OR Int (0 = use default white texture)
                        let tex_id: Option<i64> = match &args[1] {
                            RelType::Handle(crate::executor::NativeHandle(t)) => Some(*t),
                            RelType::Int(i) => Some(*i),
                            _ => None,
                        };
                        if let RelType::Handle(crate::executor::NativeHandle(win)) = &args[0]
                            && let Some(tex) = tex_id
                            && let (Some(w), Some(h), Some(d), Some(x), Some(y), Some(z)) = (
                                get_float(&args[2]),
                                get_float(&args[3]),
                                get_float(&args[4]),
                                get_float(&args[5]),
                                get_float(&args[6]),
                                get_float(&args[7]),
                            )
                        {
                            let id = crate::natives::registry::registry_spawn_cube(
                                *win, tex, w, h, d, x, y, z,
                            );
                            return Some(ExecResult::Value(RelType::Int(id)));
                        } else {
                            return Some(ExecResult::Fault {
                                msg: "[FFI] registry_spawn_cube type error: arg 1 must be Handle(win), arg 2 Handle or Int(tex)".to_string(),
                                node: "Native::Bridge::registry_spawn_cube".into()
                            });
                        }
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_spawn_cube expects (Handle win, Handle|Int tex, Float w, Float h, Float d, Float x, Float y, Float z)".to_string(),
                        node: "Native::Bridge::registry_spawn_cube".into()
                    })
                }
                "registry_spawn_sphere" => {
                    if args.len() == 8 {
                        let get_float = |arg: &RelType| -> Option<f32> {
                            match arg {
                                RelType::Float(f) => Some(*f as f32),
                                RelType::Int(i) => Some(*i as f32),
                                _ => None,
                            }
                        };
                        let get_int = |arg: &RelType| -> Option<i64> {
                            match arg {
                                RelType::Int(i) => Some(*i),
                                _ => None,
                            }
                        };

                        // tex may be a proper Handle OR Int (0 = use default white texture)
                        let tex_id: Option<i64> = match &args[1] {
                            RelType::Handle(crate::executor::NativeHandle(t)) => Some(*t),
                            RelType::Int(i) => Some(*i),
                            _ => None,
                        };

                        if let RelType::Handle(crate::executor::NativeHandle(win)) = &args[0]
                            && let Some(tex) = tex_id
                            && let (Some(r), Some(rings), Some(sectors), Some(x), Some(y), Some(z)) = (
                                get_float(&args[2]),
                                get_int(&args[3]),
                                get_int(&args[4]),
                                get_float(&args[5]),
                                get_float(&args[6]),
                                get_float(&args[7]),
                            )
                        {
                            let id = crate::natives::registry::registry_spawn_sphere(
                                *win,
                                tex,
                                r,
                                rings as i32,
                                sectors as i32,
                                x,
                                y,
                                z,
                            );
                            return Some(ExecResult::Value(RelType::Int(id)));
                        } else {
                            return Some(ExecResult::Fault {
                                msg: "[FFI] registry_spawn_sphere type error: arg 1 must be Handle(win), arg 2 Handle or Int(tex)"
                                    .to_string(),
                                node: "Native::Bridge::registry_spawn_sphere".into(),
                            });
                        }
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_spawn_sphere expects (Handle win, Handle|Int tex, Float r, Int rings, Int sectors, Float x, Float y, Float z)".to_string(),
                        node: "Native::Bridge::registry_spawn_sphere".into()
                    })
                }
                "registry_spawn_cylinder" => {
                    if args.len() == 8 {
                        let get_float = |arg: &RelType| -> Option<f32> {
                            match arg {
                                RelType::Float(f) => Some(*f as f32),
                                RelType::Int(i) => Some(*i as f32),
                                _ => None,
                            }
                        };
                        let get_int = |arg: &RelType| -> Option<i64> {
                            match arg {
                                RelType::Int(i) => Some(*i),
                                _ => None,
                            }
                        };

                        // tex may be a proper Handle OR Int (0 = use default white texture)
                        let tex_id: Option<i64> = match &args[1] {
                            RelType::Handle(crate::executor::NativeHandle(t)) => Some(*t),
                            RelType::Int(i) => Some(*i),
                            _ => None,
                        };

                        if let RelType::Handle(crate::executor::NativeHandle(win)) = &args[0]
                            && let Some(tex) = tex_id
                            && let (Some(r), Some(h), Some(s), Some(x), Some(y), Some(z)) = (
                                get_float(&args[2]),
                                get_float(&args[3]),
                                get_int(&args[4]),
                                get_float(&args[5]),
                                get_float(&args[6]),
                                get_float(&args[7]),
                            )
                        {
                            let id = crate::natives::registry::registry_spawn_cylinder(
                                *win, tex, r, h, s as i32, x, y, z,
                            );
                            return Some(ExecResult::Value(RelType::Int(id)));
                        } else {
                            return Some(ExecResult::Fault {
                                msg: "[FFI] registry_spawn_cylinder type error: arg 1 must be Handle(win), arg 2 Handle or Int(tex)"
                                    .to_string(),
                                node: "Native::Bridge::registry_spawn_cylinder".into(),
                            });
                        }
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_spawn_cylinder expects (Handle win, Handle|Int tex, Float r, Float h, Int segments, Float x, Float y, Float z)".to_string(),
                        node: "Native::Bridge::registry_spawn_cylinder".into()
                    })
                }
                "registry_update_entity_transform" => {
                    if args.len() == 5 {
                        let get_float = |arg: &RelType| -> Option<f32> {
                            match arg {
                                RelType::Float(f) => Some(*f as f32),
                                RelType::Int(i) => Some(*i as f32),
                                _ => None,
                            }
                        };
                        if let (
                            RelType::Handle(crate::executor::NativeHandle(win)),
                            RelType::Int(entity_id),
                        ) = (&args[0], &args[1])
                            && let (Some(x), Some(y), Some(z)) = (
                                get_float(&args[2]),
                                get_float(&args[3]),
                                get_float(&args[4]),
                            )
                        {
                            crate::natives::registry::registry_update_entity_transform(
                                *win, *entity_id, x, y, z,
                            );
                            return Some(ExecResult::Value(RelType::Void));
                        } else {
                            return Some(ExecResult::Fault {
                                msg: "[FFI] registry_update_entity_transform type error: coordinates must be numeric".to_string(),
                                node: "Native::Bridge::registry_update_entity_transform".into()
                            });
                        }
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_update_entity_transform expects (Handle win, Int entity, Float x, Float y, Float z)".to_string(),
                        node: "Native::Bridge::registry_update_entity_transform".into()
                    })
                }
                "registry_destroy_entity" => {
                    if args.len() == 2
                        && let RelType::Handle(_) = &args[0]
                        && let RelType::Int(entity_id) = &args[1]
                    {
                        let win_handle = match &args[0] {
                            RelType::Handle(crate::executor::NativeHandle(h)) => *h,
                            _ => -1,
                        };
                        crate::natives::registry::registry_destroy_entity(win_handle, *entity_id);
                        return Some(ExecResult::Value(RelType::Void));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_destroy_entity expects (Handle win, Int entity_id)"
                            .to_string(),
                        node: "Native::Bridge::registry_destroy_entity".into(),
                    })
                }
                "registry_set_camera" => {
                    if args.len() == 4 {
                        let get_float = |arg: &RelType| -> Option<f32> {
                            match arg {
                                RelType::Float(f) => Some(*f as f32),
                                RelType::Int(i) => Some(*i as f32),
                                _ => None,
                            }
                        };
                        if let (Some(fov), Some(x), Some(y), Some(z)) = (
                            get_float(&args[0]),
                            get_float(&args[1]),
                            get_float(&args[2]),
                            get_float(&args[3]),
                        ) {
                            crate::natives::registry::registry_set_camera(fov, x, y, z);
                            return Some(ExecResult::Value(RelType::Void));
                        }
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_set_camera expects (Float fov, Float x, Float y, Float z)"
                            .to_string(),
                        node: "Native::Bridge::registry_set_camera".into()
                    })
                }
                // Sprint 86: window-specific camera — (Handle win, Float fov, Float x, Float y, Float z)
                "registry_set_camera_for_window" => {
                    if args.len() == 5 {
                        let get_float = |arg: &RelType| -> Option<f32> {
                            match arg {
                                RelType::Float(f) => Some(*f as f32),
                                RelType::Int(i) => Some(*i as f32),
                                _ => None,
                            }
                        };
                        if let RelType::Handle(crate::executor::NativeHandle(win_id)) = &args[0]
                            && let (Some(fov), Some(x), Some(y), Some(z)) = (
                                get_float(&args[1]),
                                get_float(&args[2]),
                                get_float(&args[3]),
                                get_float(&args[4]),
                            )
                        {
                            crate::natives::registry::registry_set_camera_for_window(
                                *win_id, fov, x, y, z,
                            );
                            return Some(ExecResult::Value(RelType::Void));
                        }
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_set_camera_for_window expects (Handle win, Float fov, Float x, Float y, Float z)"
                            .to_string(),
                        node: "Native::Bridge::registry_set_camera_for_window".into()
                    })
                }

                "registry_get_mouse_delta_x" => {
                    if args.is_empty() {
                        let dx = crate::natives::registry::registry_get_mouse_delta_x();
                        return Some(ExecResult::Value(RelType::Float(dx as f64)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_get_mouse_delta_x expects 0 args".to_string(),
                        node: "Native::Bridge::registry_get_mouse_delta_x".into(),
                    })
                }
                "registry_get_mouse_delta_y" => {
                    if args.is_empty() {
                        let dy = crate::natives::registry::registry_get_mouse_delta_y();
                        return Some(ExecResult::Value(RelType::Float(dy as f64)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_get_mouse_delta_y expects 0 args".to_string(),
                        node: "Native::Bridge::registry_get_mouse_delta_y".into(),
                    })
                }
                "registry_get_last_char" => {
                    if args.is_empty() {
                        let c = crate::natives::registry::registry_get_last_char();
                        return Some(ExecResult::Value(RelType::Int(c)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_get_last_char expects 0 args".to_string(),
                        node: "Native::Bridge::registry_get_last_char".into(),
                    })
                }
                // Sprint 163: Parse a String to Float for UI→3D value bridging.
                // Returns 0.0 on invalid input (never faults — safe for live text fields).
                #[cfg(debug_assertions)]
                "registry_force_panic" => {
                    crate::natives::registry::registry_force_panic();
                    Some(ExecResult::Value(RelType::Void))
                }
                "registry_parse_float" => {
                    if args.len() == 1
                        && let RelType::Str(s) = &args[0]
                    {
                        let val: f64 = s.trim().parse().unwrap_or(0.0);
                        return Some(ExecResult::Value(RelType::Float(val)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_parse_float expects (String)".to_string(),
                        node: "Native::Bridge::registry_parse_float".into(),
                    })
                }
                // Sprint 164: Physics & Raycasting FFI bindings
                "registry_check_collision" => {
                    if args.len() == 2
                        && let (RelType::Int(id1), RelType::Int(id2)) = (&args[0], &args[1])
                    {
                        return Some(ExecResult::Value(RelType::Bool(
                            crate::natives::registry::registry_check_collision(*id1, *id2),
                        )));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_check_collision expects (Int, Int)".to_string(),
                        node: "Native::Bridge::registry_check_collision".into(),
                    })
                }
                "registry_get_clicked_entity" => {
                    if args.len() == 1
                        && let RelType::Handle(crate::executor::NativeHandle(win)) = &args[0]
                    {
                        return Some(ExecResult::Value(RelType::Int(
                            crate::natives::registry::registry_get_clicked_entity(*win),
                        )));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_get_clicked_entity expects (Handle win)".to_string(),
                        node: "Native::Bridge::registry_get_clicked_entity".into(),
                    })
                }
                // Sprint 165: Texture Loading
                "registry_load_texture" => {
                    if !permissions.allow_fs_read {
                        return Some(ExecResult::Fault {
                            msg:
                                "Permission Denied: registry.registry_load_texture requires FS_READ"
                                    .to_string(),
                            node: "Bridge::registry.registry_load_texture".into(),
                        });
                    }
                    if args.len() == 1
                        && let RelType::Str(path) = &args[0]
                    {
                        if crate::natives::ffi_safety::validate_string(
                            path,
                            "registry_load_texture",
                        )
                        .is_none()
                        {
                            return Some(ExecResult::Fault {
                                msg: "[FFI] registry_load_texture: invalid path string".to_string(),
                                node: "Native::Bridge::registry_load_texture".into(),
                            });
                        }
                        return Some(ExecResult::Value(RelType::Int(
                            crate::natives::registry::registry_load_texture(path),
                        )));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_load_texture expects (String path)".to_string(),
                        node: "Native::Bridge::registry_load_texture".into(),
                    })
                }
                // Sprint 167: Dynamic Lighting
                "registry_spawn_light" => {
                    if args.len() == 5
                        && let RelType::Handle(crate::executor::NativeHandle(win)) = &args[0]
                        && let RelType::Float(x) = &args[1]
                        && let RelType::Float(y) = &args[2]
                        && let RelType::Float(z) = &args[3]
                        && let RelType::Float(intensity) = &args[4]
                    {
                        return Some(ExecResult::Value(RelType::Int(
                            crate::natives::registry::registry_spawn_light(
                                *win,
                                *x as f32,
                                *y as f32,
                                *z as f32,
                                1.0,
                                1.0,
                                1.0,
                                *intensity as f32,
                            ),
                        )));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_spawn_light expects (Handle win, Float x, Float y, Float z, Float intensity)".to_string(),
                        node: "Native::Bridge::registry_spawn_light".into(),
                    })
                }
                "registry_spawn_light_rgb" => {
                    if args.len() == 8
                        && let RelType::Handle(crate::executor::NativeHandle(win)) = &args[0]
                        && let RelType::Float(x) = &args[1]
                        && let RelType::Float(y) = &args[2]
                        && let RelType::Float(z) = &args[3]
                        && let RelType::Float(r) = &args[4]
                        && let RelType::Float(g) = &args[5]
                        && let RelType::Float(b) = &args[6]
                        && let RelType::Float(intensity) = &args[7]
                    {
                        return Some(ExecResult::Value(RelType::Int(
                            crate::natives::registry::registry_spawn_light(
                                *win,
                                *x as f32,
                                *y as f32,
                                *z as f32,
                                *r as f32,
                                *g as f32,
                                *b as f32,
                                *intensity as f32,
                            ),
                        )));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_spawn_light_rgb expects (Handle win, Float x, Float y, Float z, Float r, Float g, Float b, Float intensity)".to_string(),
                        node: "Native::Bridge::registry_spawn_light_rgb".into(),
                    })
                }
                "registry_update_light_position" => {
                    if args.len() == 5
                        && let RelType::Handle(crate::executor::NativeHandle(win)) = &args[0]
                        && let RelType::Int(light_id) = &args[1]
                        && let RelType::Float(x) = &args[2]
                        && let RelType::Float(y) = &args[3]
                        && let RelType::Float(z) = &args[4]
                    {
                        crate::natives::registry::registry_update_light_position(
                            *win, *light_id, *x as f32, *y as f32, *z as f32,
                        );
                        return Some(ExecResult::Value(RelType::Void));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_update_light_position expects (Handle win, Int light_id, Float x, Float y, Float z)".to_string(),
                        node: "Native::Bridge::registry_update_light_position".into(),
                    })
                }
                _ => None,
            }
        } else if module == "math" {
            match function {
                "math_sin" => {
                    if args.len() == 1
                        && let RelType::Float(v) = &args[0]
                    {
                        return Some(ExecResult::Value(RelType::Float(v.sin())));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] math_sin expects (Float)".to_string(),
                        node: "Native::Bridge::math_sin".into(),
                    })
                }
                "math_cos" => {
                    if args.len() == 1
                        && let RelType::Float(v) = &args[0]
                    {
                        return Some(ExecResult::Value(RelType::Float(v.cos())));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] math_cos expects (Float)".to_string(),
                        node: "Native::Bridge::math_cos".into(),
                    })
                }
                "math_tan" => {
                    if args.len() == 1
                        && let RelType::Float(v) = &args[0]
                    {
                        return Some(ExecResult::Value(RelType::Float(v.tan())));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] math_tan expects (Float)".to_string(),
                        node: "Native::Bridge::math_tan".into(),
                    })
                }
                "math_sqrt" => {
                    if args.len() == 1
                        && let RelType::Float(v) = &args[0]
                    {
                        return Some(ExecResult::Value(RelType::Float(v.sqrt())));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] math_sqrt expects (Float)".to_string(),
                        node: "Native::Bridge::math_sqrt".into(),
                    })
                }
                "math_abs" => {
                    if args.len() == 1
                        && let RelType::Float(v) = &args[0]
                    {
                        return Some(ExecResult::Value(RelType::Float(v.abs())));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] math_abs expects (Float)".to_string(),
                        node: "Native::Bridge::math_abs".into(),
                    })
                }
                "math_pi" => {
                    if args.is_empty() {
                        return Some(ExecResult::Value(RelType::Float(std::f64::consts::PI)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] math_pi expects 0 arguments".to_string(),
                        node: "Native::Bridge::math_pi".into(),
                    })
                }
                "math_random" => {
                    if args.len() == 2
                        && let RelType::Float(min_val) = &args[0]
                        && let RelType::Float(max_val) = &args[1]
                    {
                        let r: f64 = rand::random::<f64>();
                        let result = min_val + (max_val - min_val) * r;
                        return Some(ExecResult::Value(RelType::Float(result)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] math_random expects (Float min, Float max)".to_string(),
                        node: "Native::Bridge::math_random".into(),
                    })
                }
                // Sprint 187: Parallel vector scale — multiply each element by a factor
                "math_vector_scale" => {
                    if args.len() == 2
                        && let RelType::Array(arr) = &args[0]
                        && let RelType::Float(factor) = &args[1]
                    {
                        let scaled: Vec<RelType> = arr
                            .iter()
                            .map(|elem| match elem {
                                RelType::Float(f) => RelType::Float(f * factor),
                                RelType::Int(i) => RelType::Float(*i as f64 * factor),
                                _ => elem.clone(),
                            })
                            .collect();
                        return Some(ExecResult::Value(RelType::Array(scaled)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] math_vector_scale expects (Array, Float)".to_string(),
                        node: "Native::Bridge::math_vector_scale".into(),
                    })
                }
                // Sprint 187: 4×4 matrix-vector transformation using glam
                "math_matrix_transform" => {
                    if args.len() == 2
                        && let RelType::Array(matrix_arr) = &args[0]
                        && let RelType::Array(vector_arr) = &args[1]
                    {
                        if matrix_arr.len() == 16 && (vector_arr.len() == 3 || vector_arr.len() == 4) {
                            let extract = |v: &RelType| match v {
                                RelType::Float(f) => *f as f32,
                                RelType::Int(i) => *i as f32,
                                _ => 0.0_f32,
                            };
                            let m = glam::Mat4::from_cols_array_2d(&[
                                [extract(&matrix_arr[0]), extract(&matrix_arr[1]), extract(&matrix_arr[2]), extract(&matrix_arr[3])],
                                [extract(&matrix_arr[4]), extract(&matrix_arr[5]), extract(&matrix_arr[6]), extract(&matrix_arr[7])],
                                [extract(&matrix_arr[8]), extract(&matrix_arr[9]), extract(&matrix_arr[10]), extract(&matrix_arr[11])],
                                [extract(&matrix_arr[12]), extract(&matrix_arr[13]), extract(&matrix_arr[14]), extract(&matrix_arr[15])],
                            ]);
                            let v = glam::Vec4::new(
                                extract(&vector_arr[0]),
                                extract(&vector_arr[1]),
                                extract(&vector_arr[2]),
                                if vector_arr.len() == 4 { extract(&vector_arr[3]) } else { 1.0 },
                            );
                            let result = m * v;
                            return Some(ExecResult::Value(RelType::Array(vec![
                                RelType::Float(result.x as f64),
                                RelType::Float(result.y as f64),
                                RelType::Float(result.z as f64),
                                RelType::Float(result.w as f64),
                            ])));
                        }
                        return Some(ExecResult::Fault {
                            msg: "[FFI] math_matrix_transform: matrix must be 16 floats, vector 3–4 floats".to_string(),
                            node: "Native::Bridge::math_matrix_transform".into(),
                        });
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] math_matrix_transform expects (Array[16], Array[3-4])".to_string(),
                        node: "Native::Bridge::math_matrix_transform".into(),
                    })
                }
                _ => None,
            }
        } else if module == "string" {
            match function {
                "string_len" => {
                    if args.len() == 1
                        && let RelType::Str(s) = &args[0]
                    {
                        let len = s.chars().count() as i64;
                        return Some(ExecResult::Value(RelType::Int(len)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] string_len expects 1 String arg".to_string(),
                        node: "Native::Bridge::string_len".into(),
                    })
                }
                "string_concat" => {
                    if args.len() == 2
                        && let (RelType::Str(a), RelType::Str(b)) = (&args[0], &args[1])
                    {
                        return Some(ExecResult::Value(RelType::Str(a.clone() + b)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] string_concat expects (String, String)".to_string(),
                        node: "Native::Bridge::string_concat".into(),
                    })
                }
                "string_split" => {
                    if args.len() == 2
                        && let (RelType::Str(s), RelType::Str(delim)) = (&args[0], &args[1])
                    {
                        let parts: Vec<RelType> = s
                            .split(delim.as_str())
                            .map(|part| RelType::Str(part.to_string()))
                            .collect();
                        return Some(ExecResult::Value(RelType::Array(parts)));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] string_split expects (String, String)".to_string(),
                        node: "Native::Bridge::string_split".into(),
                    })
                }
                "string_to_upper" => {
                    if args.len() == 1
                        && let RelType::Str(s) = &args[0]
                    {
                        return Some(ExecResult::Value(RelType::Str(s.to_uppercase())));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] string_to_upper expects 1 String arg".to_string(),
                        node: "Native::Bridge::string_to_upper".into(),
                    })
                }
                _ => None,
            }
        } else if module == "wgpu" {
            match function {
                "load_compute_shader" => {
                    if args.len() == 1
                        && let RelType::Str(source) = &args[0]
                    {
                        let id =
                            crate::natives::registry::registry_load_compute_shader(source.clone());
                        return Some(ExecResult::Value(RelType::Handle(
                            crate::executor::NativeHandle(id),
                        )));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] load_compute_shader expects 1 String arg (source)".to_string(),
                        node: "Native::Bridge::load_compute_shader".into(),
                    })
                }
                "dispatch_compute" => {
                    if args.len() >= 4 {
                        let shader_id = match &args[0] {
                            RelType::Handle(crate::executor::NativeHandle(id)) => *id,
                            _ => {
                                return Some(ExecResult::Fault {
                                    msg: "dispatch_compute: shader must be a Handle".into(),
                                    node: "Native::Bridge::dispatch_compute".into(),
                                });
                            }
                        };
                        let x = match &args[1] {
                            RelType::Int(i) => *i as u32,
                            RelType::Float(f) => *f as u32,
                            _ => 1,
                        };
                        let y = match &args[2] {
                            RelType::Int(i) => *i as u32,
                            RelType::Float(f) => *f as u32,
                            _ => 1,
                        };
                        let z = match &args[3] {
                            RelType::Int(i) => *i as u32,
                            RelType::Float(f) => *f as u32,
                            _ => 1,
                        };

                        let inputs = args[4..].to_vec();
                        crate::natives::registry::registry_dispatch_compute(
                            shader_id, x, y, z, inputs,
                        );
                        return Some(ExecResult::Value(RelType::Void));
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] dispatch_compute expects at least 4 args (shader, x, y, z, ...inputs)".to_string(),
                        node: "Native::Bridge::dispatch_compute".into(),
                    })
                }
                _ => None,
            }
        } else {
            None
        }
    }
}
