import json
import os

os.makedirs("benchmark/tasks", exist_ok=True)

tasks = [
  ("01_assign", "Weise der Variable x den Wert 42 zu und gib ihn aus.", "42 auf stdout", "PASS wenn 42 ausgegeben wird."),
  ("02_arithmetic", "Addiere zwei Zahlen und gib das Ergebnis aus.", "Summe auf stdout", "PASS wenn das Ergebnis numerisch korrekt ausgegeben wird."),
  ("03_concat", "Erstelle einen String und konkateniere ihn mit einem zweiten.", "Zusammengesetzter String auf stdout", "PASS wenn beide Strings aneinandergereiht ausgegeben werden."),
  ("04_if_condition", "Schreibe eine If-Bedingung die 'yes' oder 'no' ausgibt.", "'yes' oder 'no' auf stdout", "PASS wenn eine funktionierende if/else Verzweigung existiert."),
  ("05_array", "Erstelle ein Array mit drei Integers und gib das zweite aus.", "Zweiter Integer auf stdout", "PASS wenn der zweite Integer (Index 1) auf stdout steht."),
  ("06_while", "Zähle mit einer While-Schleife von 1 bis 10.", "Zahlen 1 bis 10 auf stdout", "PASS wenn alle 10 Zahlen sequenziell ausgegeben werden."),
  ("07_max_if_else", "Finde das Maximum zweier Zahlen mit If/Else.", "Das Maximum auf stdout", "PASS wenn die größere Zahl korrekt auf Basis einer Bedingung identifiziert und ausgegeben wird."),
  ("08_sum_100", "Summiere alle Zahlen von 1 bis 100 in einer Schleife.", "5050 auf stdout", "PASS wenn exakt 5050 geprintet wird."),
  ("09_fizzbuzz", "Implementiere FizzBuzz für 1-20 (Print statt UI).", "FizzBuzz-Folge auf stdout", "PASS wenn FizzBuzz-Regeln von 1 bis 20 korrekt befolgt und ausgegeben werden."),
  ("10_read_file", "Lese eine Datei ('README.md') und gib ihren Inhalt aus. (--allow-read)", "Dateiinhalt auf stdout", "PASS wenn fs_read_file oder FileRead korrekt implementiert und der Inhalt geprintet wird."),
  ("11_window_3s", "Erstelle ein Fenster und halte es 3 Sekunden offen.", "WGPU Fenster für ca. 3s sichtbar", "PASS wenn registry_create_window, registry_window_update und registry_elapsed_ms/time_sleep_ms korrekt integriert sind."),
  ("12_fetch_json", "Fetch die jsonplaceholder API und gib den title-Wert aus.", "Title-String auf stdout", "PASS wenn Fetch oder net_fetch mit fs_parse_json und obj_get korrekt kombiniert wird."),
  ("13_write_file", "Schreibe 'Hello KnotenCore' in eine Datei. (--allow-write)", "Datei auf der Festplatte mit Inhalt erstellt", "PASS wenn fs_write_file, registry_file_create/write oder FileWrite eingesetzt und Hello KnotenCore erfolgreich auf HDD landet."),
  ("14_ui_label_button", "Erstelle ein UI mit einem Label und einem Button.", "UI Fenster mit Label und Button sichtbar", "PASS wenn UIWindow, UILabel und UIButton in gültiger Kaskade verschachtelt sind."),
  ("15_perf_timer", "Messe die Ausführungszeit mit registry_now/registry_elapsed_ms.", "Gemessene Ms auf stdout", "PASS wenn Start-Timer und Elapsed-Zeit kombiniert und als Float/Int geprintet werden."),
  ("16_file_pipeline", "Baue die kanonische File Pipeline (Sprint 125, Aufgabe 4) - Lese Datei, modifiziere Properties, schreibe Datei neu.", "Modifizierte Datenstruktur auf Festplatte", "PASS wenn Read, Parse, Modify Property und Write korrekt ablaufen."),
  ("17_dashboard", "Baue das kanonische Data Dashboard (Sprint 125, Aufgabe 3) - Rendere ein Array von Daten als UILabels in einer UIVBox.", "UI mit iterierten Labels in Box sichtbar", "PASS wenn Schleifenkonstrukt mit UI-Instanziierungen in einer UIVBox verschachtelt wird."),
  ("18_calculator", "Implementiere einen einfachen Taschenrechner als egui-UI.", "Taschenrechner-App sichtbar", "PASS wenn UIButton Aktionen valide Modifikationen eines globalen Variablen-States durchführen."),
  ("19_fetch_parse_write", "Fetch + Parse + Schreibe das Ergebnis in eine Datei.", "API Result in Datei geschrieben", "PASS wenn Fetch/Net, JSON-Parse und Schreiben kombiniert ausgeführt werden."),
  ("20_minimal_window_fps", "Baue das kanonische Minimal Window mit FPS-Anzeige als Label.", "Egui Window mit FPS-Update", "PASS wenn eine Endlosschleife Render-Frames berechnet und die Elapsed-Time auf ein FPS Label mappt.")
]

for idx, (name, desc, expected, criteria) in enumerate(tasks, 1):
    d = {"description": desc, "expected_output": expected, "evaluation_criteria": criteria}
    with open(f"benchmark/tasks/{idx:02d}_{name}.json", "w", encoding="utf-8") as f:
        json.dump(d, f, indent=2, ensure_ascii=False)
    with open(f"benchmark/tasks/{idx:02d}_{name}.nod", "w", encoding="utf-8") as f:
        f.write("") # Create empty file to be filled during baseline evaluation
