import json
import os

nods = {
  "01_assign": {"Block": [
    {"Assign": ["x", {"IntLiteral": 42}]},
    {"Print": {"Identifier": "x"}}
  ]},
  "02_arithmetic": {"Block": [
    {"Print": {"Add": [{"IntLiteral": 20}, {"IntLiteral": 22}]}}
  ]},
  "03_concat": {"Block": [
    {"Print": {"Concat": [{"StringLiteral": "Hello "}, {"StringLiteral": "KnotenCore"}]}}
  ]},
  "04_if_condition": {"If": [
    {"BoolLiteral": True},
    {"Print": {"StringLiteral": "yes"}},
    {"Print": {"StringLiteral": "no"}}
  ]},
  "05_array": {"Block": [
    {"Assign": ["arr", {"ArrayCreate": [{"IntLiteral": 10}, {"IntLiteral": 20}, {"IntLiteral": 30}]}]},
    {"Print": {"ArrayGet": [{"Identifier": "arr"}, {"IntLiteral": 1}]}}
  ]},
  "06_while": {"Block": [
    {"Assign": ["i", {"IntLiteral": 1}]},
    {"While": [
      {"Lte": [{"Identifier": "i"}, {"IntLiteral": 10}]},
      {"Block": [
        {"Print": {"Identifier": "i"}},
        {"Assign": ["i", {"Add": [{"Identifier": "i"}, {"IntLiteral": 1}]}]}
      ]}
    ]}
  ]},
  "07_max_if_else": {"Block": [
    {"Assign": ["a", {"IntLiteral": 5}]},
    {"Assign": ["b", {"IntLiteral": 10}]},
    {"If": [
      {"Gt": [{"Identifier": "a"}, {"Identifier": "b"}]},
      {"Print": {"Identifier": "a"}},
      {"Print": {"Identifier": "b"}}
    ]}
  ]},
  "08_sum_100": {"Block": [
    {"Assign": ["sum", {"IntLiteral": 0}]},
    {"Assign": ["i", {"IntLiteral": 1}]},
    {"While": [
      {"Lte": [{"Identifier": "i"}, {"IntLiteral": 100}]},
      {"Block": [
        {"Assign": ["sum", {"Add": [{"Identifier": "sum"}, {"Identifier": "i"}]}]},
        {"Assign": ["i", {"Add": [{"Identifier": "i"}, {"IntLiteral": 1}]}]}
      ]}
    ]},
    {"Print": {"Identifier": "sum"}}
  ]},
  "09_fizzbuzz": {"Block": [
    {"Assign": ["i", {"IntLiteral": 1}]},
    {"While": [
      {"Lte": [{"Identifier": "i"}, {"IntLiteral": 20}]},
      {"Block": [
        {"If": [
          {"And": [
            {"Eq": [{"Sub": [{"Identifier": "i"}, {"Mul": [{"Div": [{"Identifier": "i"}, {"IntLiteral": 3}]}, {"IntLiteral": 3}]}]}, {"IntLiteral": 0}]},
            {"Eq": [{"Sub": [{"Identifier": "i"}, {"Mul": [{"Div": [{"Identifier": "i"}, {"IntLiteral": 5}]}, {"IntLiteral": 5}]}]}, {"IntLiteral": 0}]}
          ]},
          {"Print": {"StringLiteral": "FizzBuzz"}},
          {"If": [
            {"Eq": [{"Sub": [{"Identifier": "i"}, {"Mul": [{"Div": [{"Identifier": "i"}, {"IntLiteral": 3}]}, {"IntLiteral": 3}]}]}, {"IntLiteral": 0}]},
            {"Print": {"StringLiteral": "Fizz"}},
            {"If": [
              {"Eq": [{"Sub": [{"Identifier": "i"}, {"Mul": [{"Div": [{"Identifier": "i"}, {"IntLiteral": 5}]}, {"IntLiteral": 5}]}]}, {"IntLiteral": 0}]},
              {"Print": {"StringLiteral": "Buzz"}},
              {"Print": {"Identifier": "i"}}
            ]}
          ]}
        ]},
        {"Assign": ["i", {"Add": [{"Identifier": "i"}, {"IntLiteral": 1}]}]}
      ]}
    ]}
  ]},
  "10_read_file": {"Block": [
    {"Print": {"FileRead": {"StringLiteral": "README.md"}}}
  ]},
  "11_window_3s": {"Block": [
    {"Assign": ["win", {"ExternCall": {"module": "registry", "function": "registry_create_window", "args": [{"IntLiteral": 400}, {"IntLiteral": 300}, {"StringLiteral": "Test Window"}]}}]},
    {"Assign": ["timer", {"ExternCall": {"module": "registry", "function": "registry_now", "args": []}}]},
    {"While": [
      {"And": [
        {"ExternCall": {"module": "registry", "function": "registry_window_update", "args": [{"Identifier": "win"}]}},
        {"Lte": [{"ExternCall": {"module": "registry", "function": "registry_elapsed_ms", "args": [{"Identifier": "timer"}]}}, {"IntLiteral": 3000}]}
      ]},
      {"Block": [
        {"ExternCall": {"module": "registry", "function": "registry_fill_color", "args": [{"Identifier": "win"}, {"IntLiteral": 20}, {"IntLiteral": 20}, {"IntLiteral": 40}]}},
        {"ExternCall": {"module": "time", "function": "time_sleep_ms", "args": [{"IntLiteral": 16}]}}
      ]}
    ]}
  ]},
  "12_fetch_json": {"Block": [
    {"Assign": ["res", {"ExternCall": {"module": "net", "function": "net_fetch", "args": [{"StringLiteral": "https://jsonplaceholder.typicode.com/todos/1"}]}}]},
    {"Assign": ["parsed", {"ExternCall": {"module": "fs", "function": "fs_parse_json", "args": [{"Identifier": "res"}]}}]},
    {"Print": {"ExternCall": {"module": "fs", "function": "obj_get", "args": [{"Identifier": "parsed"}, {"StringLiteral": "title"}]}}}
  ]},
  "13_write_file": {"Block": [
    {"FileWrite": [{"StringLiteral": "benchmark/tasks/test_out.txt"}, {"StringLiteral": "Hello KnotenCore"}]}
  ]},
  "14_ui_label_button": {"Block": [
    {"UIWindow": ["wnd_id", {"StringLiteral": "UI Test"}, {"Block": [
      {"UILabel": {"StringLiteral": "Hello UI"}},
      {"If": [{"UIButton": {"StringLiteral": "Click Me"}}, {"Print": {"StringLiteral": "Clicked!"}}, None]}
    ]}]}
  ]},
  "15_perf_timer": {"Block": [
    {"Assign": ["timer", {"ExternCall": {"module": "registry", "function": "registry_now", "args": []}}]},
    {"Assign": ["i", {"IntLiteral": 0}]},
    {"While": [
      {"Lt": [{"Identifier": "i"}, {"IntLiteral": 10000}]},
      {"Assign": ["i", {"Add": [{"Identifier": "i"}, {"IntLiteral": 1}]}]}
    ]},
    {"Print": {"ExternCall": {"module": "registry", "function": "registry_elapsed_ms", "args": [{"Identifier": "timer"}]}}}
  ]},
  "16_file_pipeline": {"Block": [
    {"Assign": ["data", {"StringLiteral": "{\"name\":\"Node\",\"ver\":1}"}]},
    {"FileWrite": [{"StringLiteral": "benchmark/tasks/pipe_in.json"}, {"Identifier": "data"}]},
    {"Assign": ["read_data", {"FileRead": {"StringLiteral": "benchmark/tasks/pipe_in.json"}}]},
    {"Assign": ["parsed", {"ExternCall": {"module": "fs", "function": "fs_parse_json", "args": [{"Identifier": "read_data"}]}}]},
    {"Assign": ["modified", {"ExternCall": {"module": "fs", "function": "obj_set", "args": [{"Identifier": "parsed"}, {"StringLiteral": "ver"}, {"IntLiteral": 2}]}}]},
    {"Assign": ["out_str", {"ExternCall": {"module": "json", "function": "json_stringify", "args": [{"Identifier": "modified"}]}}]},
    {"FileWrite": [{"StringLiteral": "benchmark/tasks/pipe_out.json"}, {"Identifier": "out_str"}]}
  ]},
  "17_dashboard": {"Block": [
    {"Assign": ["items", {"ArrayCreate": [{"StringLiteral": "CPU: 45%"}, {"StringLiteral": "RAM: 12GB"}, {"StringLiteral": "Net: OK"}]}]},
    {"UIWindow": ["dashboard", {"StringLiteral": "Dashboard"}, {"Block": [
      {"UIVBox": [
        {"UILabel": {"ArrayGet": [{"Identifier": "items"}, {"IntLiteral": 0}]}},
        {"UILabel": {"ArrayGet": [{"Identifier": "items"}, {"IntLiteral": 1}]}},
        {"UILabel": {"ArrayGet": [{"Identifier": "items"}, {"IntLiteral": 2}]}}
      ]}
    ]}]}
  ]},
  "18_calculator": {"Block": [
    {"Assign": ["val", {"IntLiteral": 0}]},
    {"UIWindow": ["calc", {"StringLiteral": "Calculator"}, {"Block": [
      {"UIHorizontal": {"Block": [
        {"UILabel": {"ToString": {"Identifier": "val"}}},
        {"If": [{"UIButton": {"StringLiteral": "+1"}}, {"Assign": ["val", {"Add": [{"Identifier": "val"}, {"IntLiteral": 1}]}]}, None]},
        {"If": [{"UIButton": {"StringLiteral": "-1"}}, {"Assign": ["val", {"Sub": [{"Identifier": "val"}, {"IntLiteral": 1}]}]}, None]}
      ]}}
    ]}]}
  ]},
  "19_fetch_parse_write": {"Block": [
    {"Assign": ["res", {"ExternCall": {"module": "net", "function": "net_fetch", "args": [{"StringLiteral": "https://jsonplaceholder.typicode.com/posts/1"}]}}]},
    {"Assign": ["parsed", {"ExternCall": {"module": "fs", "function": "fs_parse_json", "args": [{"Identifier": "res"}]}}]},
    {"Assign": ["title", {"ExternCall": {"module": "fs", "function": "obj_get", "args": [{"Identifier": "parsed"}, {"StringLiteral": "title"}]}}]},
    {"FileWrite": [{"StringLiteral": "benchmark/tasks/write_result.txt"}, {"Identifier": "title"}]}
  ]},
  "20_minimal_window_fps": {"Block": [
    {"Assign": ["win", {"ExternCall": {"module": "registry", "function": "registry_create_window", "args": [{"IntLiteral": 300}, {"IntLiteral": 200}, {"StringLiteral": "FPS Window"}]}}]},
    {"Assign": ["last_time", {"ExternCall": {"module": "registry", "function": "registry_now", "args": []}}]},
    {"Assign": ["fps", {"IntLiteral": 0}]},
    {"While": [
      {"And": [
        {"ExternCall": {"module": "registry", "function": "registry_window_update", "args": [{"Identifier": "win"}]}},
        {"Lte": [{"ExternCall": {"module": "registry", "function": "registry_elapsed_ms", "args": [{"Identifier": "last_time"}]}}, {"IntLiteral": 2000}]} 
      ]},
      {"Block": [
        {"ExternCall": {"module": "registry", "function": "registry_fill_color", "args": [{"Identifier": "win"}, {"IntLiteral": 10}, {"IntLiteral": 10}, {"IntLiteral": 10}]}},
        {"Assign": ["ms", {"ExternCall": {"module": "registry", "function": "registry_elapsed_ms", "args": [{"Identifier": "last_time"}]}}]},
        {"Assign": ["last_time", {"ExternCall": {"module": "registry", "function": "registry_now", "args": []}}]},
        {"If": [{"Gt": [{"Identifier": "ms"}, {"IntLiteral": 0}]}, {"Assign": ["fps", {"Div": [{"IntLiteral": 1000}, {"Identifier": "ms"}]}]}, None]},
        {"UIWindow": ["fps_wnd", {"StringLiteral": "FPS Display"}, {"Block": [
          {"UILabel": {"Concat": [{"StringLiteral": "FPS: "}, {"ToString": {"Identifier": "fps"}}]}}
        ]}]},
        {"ExternCall": {"module": "time", "function": "time_sleep_ms", "args": [{"IntLiteral": 16}]}}
      ]}
    ]}
  ]}
}

for k, v in nods.items():
    with open(f"benchmark/tasks/{k}.nod", "w", encoding="utf-8") as f:
        json.dump(v, f, indent=2, ensure_ascii=False)
