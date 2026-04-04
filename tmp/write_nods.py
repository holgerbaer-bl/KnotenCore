import json, os

tasks_dir = "benchmark/tasks"

# ── TASK FILES (description JSON) ──────────────────────────────────────────────
task_specs = [
  ("01_assign",             "Weise der Variable x den Wert 42 zu und gib ihn aus.",                     "42 auf stdout",                          "PASS wenn 42 auf stdout erscheint."),
  ("02_arithmetic",         "Addiere die Zahlen 20 und 22 und gib das Ergebnis aus.",                   "42 auf stdout",                          "PASS wenn 42 auf stdout erscheint."),
  ("03_concat",             "Erstelle zwei Strings und konkateniere sie, dann Print.",                   "'Hello KnotenCore' auf stdout",           "PASS wenn zusammengesetzter String ausgegeben wird."),
  ("04_if_condition",       "Schreibe eine If-Bedingung die 'yes' bei true und 'no' bei false ausgibt.","'yes' auf stdout",                        "PASS wenn eine funktionierende if/else Verzweigung ausgefuehrt wird."),
  ("05_array",              "Erstelle ein Array mit drei Integers [10,20,30] und gib das zweite aus.",  "20 auf stdout",                          "PASS wenn ArrayGet den korrekten Index-1-Wert ausgibt."),
  ("06_while",              "Zaehle mit einer While-Schleife von 1 bis 10 und Print jede Zahl.",        "Zahlen 1..10 auf stdout",                "PASS wenn alle 10 Zahlen sequenziell erscheinen."),
  ("07_max_if_else",        "Finde mit If/Gt das Maximum von a=5 und b=10.",                            "10 auf stdout",                          "PASS wenn die groessere Zahl ausgegeben wird."),
  ("08_sum_100",            "Summiere alle Zahlen von 1 bis 100 in einer Schleife und Print.",          "5050 auf stdout",                        "PASS wenn exakt 5050 ausgegeben wird."),
  ("09_fizzbuzz",           "FizzBuzz fuer 1-20 mit Print (keine UI).",                                 "FizzBuzz-Folge auf stdout",              "PASS wenn FizzBuzz-Regeln von 1 bis 20 korrekt ausgegeben werden."),
  ("10_read_file",          "Lese benchmark/fixtures/test_input.txt und gib Inhalt aus. (--allow-read)","'Hello KnotenCore Benchmark' auf stdout", "PASS wenn Dateiinhalt via FileRead ausgegeben wird."),
  ("11_window_3s",          "registry_create_window -> While(registry_window_update && elapsed<=3000) -> registry_window_close. Kein InitWindow!","Fenster oeffnet und schliesst sich nach ~3s", "PASS wenn korrekte Registry-Pipeline ohne InitWindow/sleep benutzt wird."),
  ("12_fetch_json",         "Fetch https://jsonplaceholder.typicode.com/todos/1 und Print den 'title' Wert. (--allow-net)","Todo-Title auf stdout","PASS wenn net_fetch + fs_parse_json + obj_get korrekt kombiniert werden."),
  ("13_write_file",         "Schreibe 'Hello KnotenCore' in einer FileWrite-Node in eine Datei. (--allow-write)","Datei benchmark/tasks/test_out.txt erstellt","PASS wenn FileWrite erfolgreich ausgefuehrt wird."),
  ("14_ui_label_button",    "Erstelle ein UIWindow mit UILabel und UIButton.",                          "Fenster mit Label+Button sichtbar",      "PASS wenn UIWindow/UILabel/UIButton korrekt verschachtelt sind."),
  ("15_perf_timer",         "Messe die Laufzeit von 10000 Iterationen mit registry_now/registry_elapsed_ms und Print.",  "Millisekunden auf stdout", "PASS wenn Timer-Start/Elapsed korrekt kombiniert und ausgegeben wird."),
  ("16_file_pipeline",      "File Pipeline: FileWrite JSON-String, FileRead, fs_parse_json, obj_set 'ver'->2, json_stringify, FileWrite Ergebnis. (--allow-read --allow-write)","pipe_out.json mit ver:2 erstellt","PASS wenn die vollstaendige Read-Modify-Write Pipeline laeuft."),
  ("17_dashboard",          "Dashboard: Drei Sensor-Strings in Array, UIWindow mit UIVBox der drei UILabels. Fenster kurz offen.",               "UI mit 3 Labels sichtbar",               "PASS wenn UIVBox mit ArrayGet-UILabels korrekt verschachtelt ist."),
  ("18_calculator",         "Rechner-GUI: UIWindow + UIHorizontal + UILabel(val) + UIButton('+1') + UIButton('-1').",                            "Rechner-App mit Buttons sichtbar",        "PASS wenn UIButton-Aktionen val modifizieren."),
  ("19_fetch_parse_write",  "Fetch jsonplaceholder/posts/1, parse JSON, obj_get 'title', FileWrite title in Datei. (--allow-net --allow-write)", "write_result.txt mit Post-Title erstellt","PASS wenn alle drei Phasen (net -> json -> fs) verbunden sind."),
  ("20_minimal_window_fps", "Minimal FPS: registry_create_window, While-Loop mit registry_now/elapsed_ms, FPS in UIWindow UILabel.",             "Fenster mit aktualisiertem FPS-Label",    "PASS wenn die Loop-Schleife FPS korrekt berechnet und im UIWindow zeigt."),
]

for name, desc, expected, criteria in task_specs:
    with open(f"{tasks_dir}/{name}.json", "w", encoding="utf-8") as f:
        json.dump({"description": desc, "expected_output": expected, "evaluation_criteria": criteria}, f, indent=2, ensure_ascii=False)

# ── NOD FILES ──────────────────────────────────────────────────────────────────
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
  # FizzBuzz: modulo simulation via subtraction (i - (i/n)*n == 0)
  "09_fizzbuzz": {"Block": [
    {"Assign": ["i", {"IntLiteral": 1}]},
    {"While": [
      {"Lte": [{"Identifier": "i"}, {"IntLiteral": 20}]},
      {"Block": [
        {"Assign": ["m3", {"Sub": [{"Identifier": "i"}, {"Mul": [{"Div": [{"Identifier": "i"}, {"IntLiteral": 3}]}, {"IntLiteral": 3}]}]}]},
        {"Assign": ["m5", {"Sub": [{"Identifier": "i"}, {"Mul": [{"Div": [{"Identifier": "i"}, {"IntLiteral": 5}]}, {"IntLiteral": 5}]}]}]},
        {"If": [
          {"And": [
            {"Eq": [{"Identifier": "m3"}, {"IntLiteral": 0}]},
            {"Eq": [{"Identifier": "m5"}, {"IntLiteral": 0}]}
          ]},
          {"Print": {"StringLiteral": "FizzBuzz"}},
          {"If": [
            {"Eq": [{"Identifier": "m3"}, {"IntLiteral": 0}]},
            {"Print": {"StringLiteral": "Fizz"}},
            {"If": [
              {"Eq": [{"Identifier": "m5"}, {"IntLiteral": 0}]},
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
    {"Print": {"FileRead": {"StringLiteral": "benchmark/fixtures/test_input.txt"}}}
  ]},
  # Task 11: proper window pipeline — NO InitWindow, NO sleep
  "11_window_3s": {"Block": [
    {"Assign": ["win", {"ExternCall": {"module": "registry", "function": "registry_create_window",
      "args": [{"IntLiteral": 640}, {"IntLiteral": 480}, {"StringLiteral": "KnotenCore - Task 11"}]}}]},
    {"Assign": ["timer", {"ExternCall": {"module": "registry", "function": "registry_now", "args": []}}]},
    {"While": [
      {"And": [
        {"ExternCall": {"module": "registry", "function": "registry_window_update", "args": [{"Identifier": "win"}]}},
        {"Lte": [
          {"ExternCall": {"module": "registry", "function": "registry_elapsed_ms", "args": [{"Identifier": "timer"}]}},
          {"IntLiteral": 3000}
        ]}
      ]},
      {"ExternCall": {"module": "registry", "function": "registry_fill_color",
        "args": [{"Identifier": "win"}, {"IntLiteral": 30}, {"IntLiteral": 10}, {"IntLiteral": 60}]}}
    ]},
    {"ExternCall": {"module": "registry", "function": "registry_window_close", "args": [{"Identifier": "win"}]}}
  ]},
  "12_fetch_json": {"Block": [
    {"Assign": ["body", {"ExternCall": {"module": "net", "function": "net_fetch",
      "args": [{"StringLiteral": "https://jsonplaceholder.typicode.com/todos/1"}]}}]},
    {"Assign": ["parsed", {"ExternCall": {"module": "fs", "function": "fs_parse_json",
      "args": [{"Identifier": "body"}]}}]},
    {"Print": {"ExternCall": {"module": "fs", "function": "obj_get",
      "args": [{"Identifier": "parsed"}, {"StringLiteral": "title"}]}}}
  ]},
  "13_write_file": {"Block": [
    {"FileWrite": [{"StringLiteral": "benchmark/tasks/test_out.txt"}, {"StringLiteral": "Hello KnotenCore"}]}
  ]},
  "14_ui_label_button": {"Block": [
    {"UIWindow": ["task14", {"StringLiteral": "Task 14 - Label & Button"}, {"Block": [
      {"UILabel": {"StringLiteral": "Hello from KnotenCore UI"}},
      {"UIButton": {"StringLiteral": "Click Me"}}
    ]}]}
  ]},
  "15_perf_timer": {"Block": [
    {"Assign": ["t0", {"ExternCall": {"module": "registry", "function": "registry_now", "args": []}}]},
    {"Assign": ["i", {"IntLiteral": 0}]},
    {"While": [
      {"Lt": [{"Identifier": "i"}, {"IntLiteral": 10000}]},
      {"Assign": ["i", {"Add": [{"Identifier": "i"}, {"IntLiteral": 1}]}]}
    ]},
    {"Print": {"ExternCall": {"module": "registry", "function": "registry_elapsed_ms",
      "args": [{"Identifier": "t0"}]}}}
  ]},
  "16_file_pipeline": {"Block": [
    {"FileWrite": [{"StringLiteral": "benchmark/tasks/pipe_in.json"}, {"StringLiteral": "{\"name\":\"Node\",\"ver\":1}"}]},
    {"Assign": ["raw", {"FileRead": {"StringLiteral": "benchmark/tasks/pipe_in.json"}}]},
    {"Assign": ["parsed", {"ExternCall": {"module": "fs", "function": "fs_parse_json", "args": [{"Identifier": "raw"}]}}]},
    {"Assign": ["modified", {"ExternCall": {"module": "fs", "function": "obj_set",
      "args": [{"Identifier": "parsed"}, {"StringLiteral": "ver"}, {"IntLiteral": 2}]}}]},
    {"Assign": ["out", {"ExternCall": {"module": "json", "function": "json_stringify", "args": [{"Identifier": "modified"}]}}]},
    {"FileWrite": [{"StringLiteral": "benchmark/tasks/pipe_out.json"}, {"Identifier": "out"}]},
    {"Print": {"StringLiteral": "Pipeline OK"}}
  ]},
  "17_dashboard": {"Block": [
    {"Assign": ["items", {"ArrayCreate": [
      {"StringLiteral": "CPU: 45%"},
      {"StringLiteral": "RAM: 12 GB"},
      {"StringLiteral": "Net: Online"}
    ]}]},
    {"UIWindow": ["dash17", {"StringLiteral": "Dashboard"}, {"Block": [
      {"UIVBox": [
        {"UILabel": {"ArrayGet": [{"Identifier": "items"}, {"IntLiteral": 0}]}},
        {"UILabel": {"ArrayGet": [{"Identifier": "items"}, {"IntLiteral": 1}]}},
        {"UILabel": {"ArrayGet": [{"Identifier": "items"}, {"IntLiteral": 2}]}}
      ]}
    ]}]}
  ]},
  "18_calculator": {"Block": [
    {"Assign": ["val", {"IntLiteral": 0}]},
    {"UIWindow": ["calc18", {"StringLiteral": "Calculator"}, {"Block": [
      {"UIHorizontal": {"Block": [
        {"UILabel": {"ToString": {"Identifier": "val"}}},
        {"If": [{"UIButton": {"StringLiteral": "+1"}},
          {"Assign": ["val", {"Add": [{"Identifier": "val"}, {"IntLiteral": 1}]}]}, None]},
        {"If": [{"UIButton": {"StringLiteral": "-1"}},
          {"Assign": ["val", {"Sub": [{"Identifier": "val"}, {"IntLiteral": 1}]}]}, None]}
      ]}}
    ]}]}
  ]},
  "19_fetch_parse_write": {"Block": [
    {"Assign": ["body", {"ExternCall": {"module": "net", "function": "net_fetch",
      "args": [{"StringLiteral": "https://jsonplaceholder.typicode.com/posts/1"}]}}]},
    {"Assign": ["parsed", {"ExternCall": {"module": "fs", "function": "fs_parse_json",
      "args": [{"Identifier": "body"}]}}]},
    {"Assign": ["title", {"ExternCall": {"module": "fs", "function": "obj_get",
      "args": [{"Identifier": "parsed"}, {"StringLiteral": "title"}]}}]},
    {"FileWrite": [{"StringLiteral": "benchmark/tasks/write_result.txt"}, {"Identifier": "title"}]},
    {"Print": {"Identifier": "title"}}
  ]},
  "20_minimal_window_fps": {"Block": [
    {"Assign": ["win", {"ExternCall": {"module": "registry", "function": "registry_create_window",
      "args": [{"IntLiteral": 400}, {"IntLiteral": 200}, {"StringLiteral": "FPS Monitor"}]}}]},
    {"Assign": ["frame_timer", {"ExternCall": {"module": "registry", "function": "registry_now", "args": []}}]},
    {"Assign": ["fps", {"IntLiteral": 0}]},
    {"Assign": ["total_timer", {"ExternCall": {"module": "registry", "function": "registry_now", "args": []}}]},
    {"While": [
      {"And": [
        {"ExternCall": {"module": "registry", "function": "registry_window_update", "args": [{"Identifier": "win"}]}},
        {"Lte": [
          {"ExternCall": {"module": "registry", "function": "registry_elapsed_ms", "args": [{"Identifier": "total_timer"}]}},
          {"IntLiteral": 3000}
        ]}
      ]},
      {"Block": [
        {"ExternCall": {"module": "registry", "function": "registry_fill_color",
          "args": [{"Identifier": "win"}, {"IntLiteral": 10}, {"IntLiteral": 10}, {"IntLiteral": 20}]}},
        {"Assign": ["ms", {"ExternCall": {"module": "registry", "function": "registry_elapsed_ms",
          "args": [{"Identifier": "frame_timer"}]}}]},
        {"Assign": ["frame_timer", {"ExternCall": {"module": "registry", "function": "registry_now", "args": []}}]},
        {"If": [{"Gt": [{"Identifier": "ms"}, {"IntLiteral": 0}]},
          {"Assign": ["fps", {"Div": [{"IntLiteral": 1000}, {"Identifier": "ms"}]}]}, None]},
        {"UIWindow": ["fps_overlay", {"StringLiteral": ""}, {"Block": [
          {"UILabel": {"Concat": [{"StringLiteral": "FPS: "}, {"ToString": {"Identifier": "fps"}}]}}
        ]}]}
      ]}
    ]},
    {"ExternCall": {"module": "registry", "function": "registry_window_close", "args": [{"Identifier": "win"}]}}
  ]}
}

for name, ast in nods.items():
    with open(f"{tasks_dir}/{name}.nod", "w", encoding="utf-8") as f:
        json.dump(ast, f, indent=2, ensure_ascii=False)

print("All 20 tasks written.")
