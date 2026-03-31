# KnotenCore AI Context Bundle v124
This document contains the complete structural DSL constraints and grammar schemas enabling self-healing deterministic generative operations.

## EBNF Grammar Specification

```ebnf
(* ============================================================ *)
(*  KnotenCore .nod Grammar — Normative Reference v1.0          *)
(*  Derived from: src/ast.rs                                     *)
(*  Sprint 121 — AI-Readiness Foundation                         *)
(*  A .nod file is a single JSON object that serialises to one   *)
(*  Node variant below. All keys are verbatim Rust enum names.   *)
(* ============================================================ *)

program     ::= node ;

node        ::= literal
              | identifier
              | assign
              | math-bin
              | math-un
              | logic-bin
              | logic-un
              | array-op
              | string-op
              | object-op
              | map-op
              | bitwise-op
              | fn-def
              | call
              | extern-call
              | native-call
              | io-node
              | control-flow
              | ui-node
              | graphics-3d
              | audio-node
              | voxel-node
              | physics-node
              | system-node
              ;

(* ─── Literals ──────────────────────────────────────────────── *)
literal     ::= int-lit | float-lit | bool-lit | string-lit ;
int-lit     ::= '{"IntLiteral":'   integer   '}' ;
float-lit   ::= '{"FloatLiteral":' number    '}' ;
bool-lit    ::= '{"BoolLiteral":'  boolean   '}' ;
string-lit  ::= '{"StringLiteral":' string   '}' ;

(* ─── Identifier / Memory ───────────────────────────────────── *)
identifier  ::= '{"Identifier":'  string '}' ;

assign      ::= '{"Assign":' '[' string ',' node ']' '}' ;

(* Persistent KV-store (Sprint 64) *)
store-node  ::= '{"Store":' '{"key":' string ',"value":' node '}}' ;
load-node   ::= '{"Load":'  '{"key":' string                  '}}' ;

(* ─── Math (binary) ─────────────────────────────────────────── *)
math-bin    ::= add | sub | mul | div | mat4-mul ;
add         ::= '{"Add":'     '[' node ',' node ']' '}' ;
sub         ::= '{"Sub":'     '[' node ',' node ']' '}' ;
mul         ::= '{"Mul":'     '[' node ',' node ']' '}' ;
div         ::= '{"Div":'     '[' node ',' node ']' '}' ;
mat4-mul    ::= '{"Mat4Mul":' '[' node ',' node ']' '}' ;

(* Math (unary) *)
math-un     ::= sin | cos | abs | neg ;
sin         ::= '{"Sin":' node '}' ;
cos         ::= '{"Cos":' node '}' ;
abs         ::= '{"Abs":' node '}' ;

(* Time constants *)
system-node ::= '{"Time":null}' | '"Time"'
              | '{"GlobalTime":null}' | '"GlobalTime"'
              | '{"InitGraphics":null}' | '"InitGraphics"'
              | '{"GetLastKeypress":null}' | '"GetLastKeypress"'
              | '{"UIFillParent":null}' | '"UIFillParent"'
              | '{"MapCreate":null}' | '"MapCreate"'
              | '{"InitAudio":null}' | '"InitAudio"'
              | '{"InitVoxelMap":null}' | '"InitVoxelMap"'
              | '{"RaycastSimple":null}' | '"RaycastSimple"'
              ;

(* ─── Logic / Comparison ────────────────────────────────────── *)
logic-bin   ::= eq | lt | gt ;
eq          ::= '{"Eq":'  '[' node ',' node ']' '}' ;
lt          ::= '{"Lt":'  '[' node ',' node ']' '}' ;
gt          ::= '{"Gt":'  '[' node ',' node ']' '}' ;

logic-un    ::= to-string | eval-json ;
to-string   ::= '{"ToString":'      node '}' ;
eval-json   ::= '{"EvalJSONNative":' node '}' ;

(* ─── Bitwise ───────────────────────────────────────────────── *)
bitwise-op  ::= bit-and | bit-shl | bit-shr ;
bit-and     ::= '{"BitAnd":'        '[' node ',' node ']' '}' ;
bit-shl     ::= '{"BitShiftLeft":'  '[' node ',' node ']' '}' ;
bit-shr     ::= '{"BitShiftRight":' '[' node ',' node ']' '}' ;

(* ─── Arrays ────────────────────────────────────────────────── *)
array-op    ::= arr-create | arr-get | arr-set | arr-push | arr-len | index ;
arr-create  ::= '{"ArrayCreate":' '[' node-list-opt ']' '}' ;
arr-get     ::= '{"ArrayGet":'    '[' node ',' node ']' '}' ;       (* array, index *)
arr-set     ::= '{"ArraySet":'    '[' node ',' node ',' node ']' '}' ; (* array, index, value *)
arr-push    ::= '{"ArrayPush":'   '[' node ',' node ']' '}' ;       (* array, value *)
arr-len     ::= '{"ArrayLen":'    node '}' ;
index       ::= '{"Index":'       '[' node ',' node ']' '}' ;       (* expr, index *)

(* ─── Strings ───────────────────────────────────────────────── *)
string-op   ::= concat ;
concat      ::= '{"Concat":' '[' node ',' node ']' '}' ;

(* ─── Objects (dot-access syntax) ───────────────────────────── *)
object-op   ::= obj-literal | prop-get | prop-set ;
obj-literal ::= '{"ObjectLiteral":' obj-map '}' ;
obj-map     ::= '{' string ':' node ( ',' string ':' node )* '}' | '{}' ;
prop-get    ::= '{"PropertyGet":' '[' node ',' string ']' '}' ;
prop-set    ::= '{"PropertySet":' '[' node ',' string ',' node ']' '}' ;

(* ─── Maps (explicit HashMap ops) ───────────────────────────── *)
map-op      ::= map-get | map-set | map-has ;
map-get     ::= '{"MapGet":'    '[' node ',' node ']' '}' ;
map-set     ::= '{"MapSet":'    '[' node ',' node ',' node ']' '}' ;
map-has     ::= '{"MapHasKey":' '[' node ',' node ']' '}' ;

(* ─── Functions ─────────────────────────────────────────────── *)
fn-def      ::= '{"FnDef":' '[' string ',' param-list ',' node ']' '}' ;
param-list  ::= '[' ( string ( ',' string )* )? ']' ;

call        ::= '{"Call":' '[' string ',' '[' node-list-opt ']' ']' '}' ;

extern-call ::= '{"ExternCall":' '{'
                  '"module":' string ','
                  '"function":' string ','
                  '"args":' '[' node-list-opt ']'
                '}}' ;

native-call ::= '{"NativeCall":' '[' string ',' '[' node-list-opt ']' ']' '}' ;

(* ─── Control Flow ───────────────────────────────────────────── *)
control-flow ::= if-node | while-node | block | return-node | import-node ;

block       ::= '{"Block":' '[' node-list-opt ']' '}' ;
if-node     ::= '{"If":'    '[' node ',' node ( ',' node )? ']' '}' ;
                (* [condition, then-branch, else-branch?] *)
while-node  ::= '{"While":' '[' node ',' node ']' '}' ;
return-node ::= '{"Return":' node '}' ;
import-node ::= '{"Import":' string '}' ;

(* ─── I/O ────────────────────────────────────────────────────── *)
io-node     ::= print | file-read | file-write | fs-read | fs-write | fetch | extract ;
print       ::= '{"Print":'     node '}' ;
file-read   ::= '{"FileRead":'  node '}' ;
file-write  ::= '{"FileWrite":' '[' node ',' node ']' '}' ;
fs-read     ::= '{"FSRead":'    node '}' ;
fs-write    ::= '{"FSWrite":'   '[' node ',' node ']' '}' ;

fetch       ::= '{"Fetch":' '{'
                  '"method":' string ','
                  '"url":' string ','
                  '"callback":' node
                '}}' ;

extract     ::= '{"Extract":' '{"source":' node ',"path":' node '}}' ;

(* ─── UI (egui) ──────────────────────────────────────────────── *)
ui-node     ::= ui-window | ui-label | ui-button | ui-text-input
              | ui-set-style | ui-horizontal | ui-hbox | ui-vbox
              | ui-fullscreen | ui-grid | ui-scroll-area
              | ui-fixed | ui-fill-parent | draw-rect ;

ui-window   ::= '{"UIWindow":' '[' string ',' node ',' node ']' '}' ;
              (* [id:String, title:node, children:Block] *)
ui-label    ::= '{"UILabel":'     node '}' ;
ui-button   ::= '{"UIButton":'    node '}' ;
              (* Evaluates to BoolLiteral(true) when clicked this frame *)
ui-text-input ::= '{"UITextInput":' node '}' ;
              (* State-binding: text = UITextInput(text)              *)
              (* Reads from thread-safe UI_TEXT_INPUT_BUFFER           *)

ui-hbox     ::= '{"UIHBox":' '[' node-list-opt ']' '}' ;
ui-vbox     ::= '{"UIVBox":' '[' node-list-opt ']' '}' ;
ui-horizontal ::= '{"UIHorizontal":' node '}' ;
ui-fullscreen ::= '{"UIFullscreen":' node '}' ;
ui-grid     ::= '{"UIGrid":' '[' integer ',' string ',' node ']' '}' ;
              (* [columns:Int, id:String, body:Block] *)
ui-scroll-area ::= '{"UIScrollArea":' '[' string ',' node ']' '}' ;
              (* [id:String, body:Block] *)
ui-fixed    ::= '{"UIFixed":' '{"width":' node ',"height":' node ',"body":' node '}}' ;
ui-fill-parent ::= '"UIFillParent"' ;

draw-rect   ::= '{"DrawRect":' '{'
                  '"x":' node ','
                  '"y":' node ','
                  '"width":' node ','
                  '"height":' node ','
                  '"color":' node
                '}}' ;

ui-set-style ::= '{"UISetStyle":' '[' node ',' node ',' node ',' node
                   ( ',' node ( ',' node )? )? ']' '}' ;
               (* [rounding, spacing, accent_rgba, fill_rgba, btn_idle?, btn_hover?] *)

(* ─── 3D Graphics ────────────────────────────────────────────── *)
graphics-3d ::= init-window | load-shader | render-mesh | poll-events
              | load-mesh | load-texture | render-asset
              | camera-3d | mesh-3d | mesh-instance | point-light | material-3d
              | fps-camera | mouse-grab | weapon-vm
              | render-canvas | transform-2d | sprite-2d
              | load-font | draw-text
              | init-camera | draw-voxel | load-tex-atlas
              ;

init-window ::= '{"InitWindow":' '[' node ',' node ',' node ']' '}' ;
load-shader ::= '{"LoadShader":' node '}' ;
render-mesh ::= '{"RenderMesh":' '[' node ',' node ',' node ']' '}' ;
poll-events ::= '{"PollEvents":' node '}' ;
load-mesh   ::= '{"LoadMesh":'   node '}' ;
load-texture ::= '{"LoadTexture":' node '}' ;
load-font   ::= '{"LoadFont":'   node '}' ;
draw-text   ::= '{"DrawText":'   '[' node ',' node ',' node ',' node ',' node ']' '}' ;
              (* [text, x, y, size, color_array] *)
render-asset ::= '{"RenderAsset":' '[' node ',' node ',' node ',' node ']' '}' ;

camera-3d   ::= '{"Camera3D":' '{'
                  '"pos_x":' node ',"pos_y":' node ',"pos_z":' node ','
                  '"target_x":' node ',"target_y":' node ',"target_z":' node ','
                  '"fov":' node
                '}}' ;

mesh-3d     ::= '{"Mesh3D":' '{"primitive":' node ',"material":' node '}}' ;
material-3d ::= '{"Material3D":' '{'
                  '"r":' node ',"g":' node ',"b":' node ',"a":' node ','
                  '"metallic":' node ',"roughness":' node
                  ( ',"texture_id":' node )? '}}' ;
point-light ::= '{"PointLight3D":' '{'
                  '"x":' node ',"y":' node ',"z":' node ','
                  '"r":' node ',"g":' node ',"b":' node ','
                  '"intensity":' node '}}' ;

mesh-instance ::= '{"MeshInstance3D":' '{'
                    '"mesh_id":' node ',"transform":' node ','
                    '"color_offset":' node ',"pbr":' node '}}' ;
fps-camera  ::= '{"FPSCamera":' '{"fov":' node '}}' ;
mouse-grab  ::= '{"MouseGrab":' '{"enabled":' node '}}' ;
weapon-vm   ::= '{"WeaponViewModel":' '{"mesh":' node ',"tex":' node '}}' ;

render-canvas ::= '{"RenderCanvas":' '{"body":' node '}}' ;
transform-2d  ::= '{"Transform2D":' '{'
                    '"x":' node ',"y":' node ',"rotation":' node ','
                    '"scale":' node ',"body":' node '}}' ;
sprite-2d   ::= '{"Sprite2D":' '{"texture_id":' node ',"transform":' node '}}' ;

(* ─── Audio ──────────────────────────────────────────────────── *)
audio-node  ::= play-note | stop-note | play-audio | load-sample | play-sample ;
play-note   ::= '{"PlayNote":'  '[' node ',' node ',' node ']' '}' ;
stop-note   ::= '{"StopNote":' node '}' ;
play-audio  ::= '{"PlayAudioFile":' node '}' ;
load-sample ::= '{"LoadSample":' '[' node ',' node ']' '}' ;
play-sample ::= '{"PlaySample":' '[' node ',' node ',' node ']' '}' ;

(* ─── Voxel Engine ───────────────────────────────────────────── *)
voxel-node  ::= init-camera | draw-voxel | load-tex-atlas
              | set-voxel | enable-interaction | enable-physics ;
init-camera ::= '{"InitCamera":' node '}' ;
draw-voxel  ::= '{"DrawVoxelGrid":' node '}' ;
load-tex-atlas ::= '{"LoadTextureAtlas":' '[' node ',' node ']' '}' ;
set-voxel   ::= '{"SetVoxel":' '[' node ',' node ',' node ',' node ']' '}' ;
enable-interaction ::= '{"EnableInteraction":' node '}' ;
enable-physics ::= '{"EnablePhysics":' node '}' ;

(* ─── Physics ────────────────────────────────────────────────── *)
physics-node ::= add-world-aabb | check-collision ;
add-world-aabb ::= '{"AddWorldAABB":' '{"min":' node ',"max":' node '}}' ;
check-collision ::= '{"CheckCollision":' '{'
                      '"a_min":' node ',"a_max":' node ','
                      '"b_min":' node ',"b_max":' node '}}' ;

(* ─── Shared helpers ─────────────────────────────────────────── *)
node-list-opt ::= ε | node ( ',' node )* ;

string  ::= '"' { any-char } '"' ;  (* JSON string *)
integer ::= [ '-' ] digit { digit } ;
number  ::= integer [ '.' digit { digit } ] ;
boolean ::= 'true' | 'false' ;
digit   ::= '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' ;

```

## Valid AST Node Types

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://github.com/holgerbaer-bl/KnotenCore/docs/LANGUAGE_REFERENCE/node_types.json",
  "title": "KnotenCore AST Node",
  "description": "Normative JSON Schema for every Node variant in KnotenCore. additionalProperties:false is enforced on every object node to eliminate AI hallucinations. Sprint 121 — v1.0.0.",
  "version": "1.0.0",
  "oneOf": [
    {
      "description": "Integer constant",
      "type": "object",
      "required": ["IntLiteral"],
      "properties": { "IntLiteral": { "type": "integer" } },
      "additionalProperties": false
    },
    {
      "description": "Floating-point constant",
      "type": "object",
      "required": ["FloatLiteral"],
      "properties": { "FloatLiteral": { "type": "number" } },
      "additionalProperties": false
    },
    {
      "description": "Boolean constant",
      "type": "object",
      "required": ["BoolLiteral"],
      "properties": { "BoolLiteral": { "type": "boolean" } },
      "additionalProperties": false
    },
    {
      "description": "String constant",
      "type": "object",
      "required": ["StringLiteral"],
      "properties": { "StringLiteral": { "type": "string" } },
      "additionalProperties": false
    },
    {
      "description": "Variable read — resolves name from engine memory",
      "type": "object",
      "required": ["Identifier"],
      "properties": { "Identifier": { "type": "string" } },
      "additionalProperties": false
    },
    {
      "description": "Variable write — [name:string, value:node]",
      "type": "object",
      "required": ["Assign"],
      "properties": {
        "Assign": {
          "type": "array",
          "prefixItems": [ { "type": "string" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Sequential execution. Returns value of last node.",
      "type": "object",
      "required": ["Block"],
      "properties": {
        "Block": { "type": "array", "items": { "$ref": "#" } }
      },
      "additionalProperties": false
    },
    {
      "description": "Conditional. else-branch is optional. [cond, then, else?]",
      "type": "object",
      "required": ["If"],
      "properties": {
        "If": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 3
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Pre-condition loop. [condition, body]",
      "type": "object",
      "required": ["While"],
      "properties": {
        "While": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Early return from function body",
      "type": "object",
      "required": ["Return"],
      "properties": { "Return": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Import a .nod or .knoten module at compile time",
      "type": "object",
      "required": ["Import"],
      "properties": { "Import": { "type": "string" } },
      "additionalProperties": false
    },
    {
      "description": "Print value to stdout",
      "type": "object",
      "required": ["Print"],
      "properties": { "Print": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Addition. [left, right]",
      "type": "object",
      "required": ["Add"],
      "properties": {
        "Add": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Subtraction. [left, right]",
      "type": "object",
      "required": ["Sub"],
      "properties": {
        "Sub": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Multiplication. [left, right]",
      "type": "object",
      "required": ["Mul"],
      "properties": {
        "Mul": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Division. Returns Fault on division by zero. [left, right]",
      "type": "object",
      "required": ["Div"],
      "properties": {
        "Div": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "4x4 matrix multiplication (3D transforms). [mat_a, mat_b] — both are Array[16 floats]",
      "type": "object",
      "required": ["Mat4Mul"],
      "properties": {
        "Mat4Mul": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Sine (radians)",
      "type": "object",
      "required": ["Sin"],
      "properties": { "Sin": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Cosine (radians)",
      "type": "object",
      "required": ["Cos"],
      "properties": { "Cos": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Absolute value",
      "type": "object",
      "required": ["Abs"],
      "properties": { "Abs": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Equality comparison. Returns Bool. [left, right]",
      "type": "object",
      "required": ["Eq"],
      "properties": {
        "Eq": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Less-than comparison. Returns Bool. [left, right]",
      "type": "object",
      "required": ["Lt"],
      "properties": {
        "Lt": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Greater-than comparison. Returns Bool. [left, right]",
      "type": "object",
      "required": ["Gt"],
      "properties": {
        "Gt": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Bitwise AND. [left, right]",
      "type": "object",
      "required": ["BitAnd"],
      "properties": {
        "BitAnd": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Bitwise left-shift. [value, shift_amount]",
      "type": "object",
      "required": ["BitShiftLeft"],
      "properties": {
        "BitShiftLeft": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Bitwise right-shift. [value, shift_amount]",
      "type": "object",
      "required": ["BitShiftRight"],
      "properties": {
        "BitShiftRight": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "String concatenation. [left, right]",
      "type": "object",
      "required": ["Concat"],
      "properties": {
        "Concat": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Convert any value to its string representation",
      "type": "object",
      "required": ["ToString"],
      "properties": { "ToString": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Array literal construction",
      "type": "object",
      "required": ["ArrayCreate"],
      "properties": {
        "ArrayCreate": { "type": "array", "items": { "$ref": "#" } }
      },
      "additionalProperties": false
    },
    {
      "description": "Array element read. [array, index:Int]",
      "type": "object",
      "required": ["ArrayGet"],
      "properties": {
        "ArrayGet": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Array element write. [array, index:Int, value]",
      "type": "object",
      "required": ["ArraySet"],
      "properties": {
        "ArraySet": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 3, "maxItems": 3
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Append element to array. [array, value]",
      "type": "object",
      "required": ["ArrayPush"],
      "properties": {
        "ArrayPush": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Get array length. Returns Int.",
      "type": "object",
      "required": ["ArrayLen"],
      "properties": { "ArrayLen": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Generic index expression. [collection, index]",
      "type": "object",
      "required": ["Index"],
      "properties": {
        "Index": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Object literal. Keys are strings, values are nodes.",
      "type": "object",
      "required": ["ObjectLiteral"],
      "properties": {
        "ObjectLiteral": {
          "type": "object",
          "additionalProperties": { "$ref": "#" }
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Read object property. [object, property_name:string]",
      "type": "object",
      "required": ["PropertyGet"],
      "properties": {
        "PropertyGet": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "type": "string" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Write object property. [object, property_name:string, value]",
      "type": "object",
      "required": ["PropertySet"],
      "properties": {
        "PropertySet": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "type": "string" }, { "$ref": "#" } ],
          "minItems": 3, "maxItems": 3
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Create an empty HashMap",
      "type": "string",
      "enum": ["MapCreate"]
    },
    {
      "description": "Read HashMap value. [map, key]",
      "type": "object",
      "required": ["MapGet"],
      "properties": {
        "MapGet": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Write HashMap value. [map, key, value]",
      "type": "object",
      "required": ["MapSet"],
      "properties": {
        "MapSet": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 3, "maxItems": 3
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Test if HashMap contains key. [map, key] → Bool",
      "type": "object",
      "required": ["MapHasKey"],
      "properties": {
        "MapHasKey": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Function definition. [name:string, params:[string…], body:node]",
      "type": "object",
      "required": ["FnDef"],
      "properties": {
        "FnDef": {
          "type": "array",
          "prefixItems": [
            { "type": "string" },
            { "type": "array", "items": { "type": "string" } },
            { "$ref": "#" }
          ],
          "minItems": 3, "maxItems": 3
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Function call. [name:string, args:[node…]]",
      "type": "object",
      "required": ["Call"],
      "properties": {
        "Call": {
          "type": "array",
          "prefixItems": [
            { "type": "string" },
            { "type": "array", "items": { "$ref": "#" } }
          ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Foreign FFI call routed through BridgeModule dispatch.",
      "type": "object",
      "required": ["ExternCall"],
      "properties": {
        "ExternCall": {
          "type": "object",
          "required": ["module", "function", "args"],
          "properties": {
            "module":   { "type": "string" },
            "function": { "type": "string" },
            "args":     { "type": "array", "items": { "$ref": "#" } }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Direct native Rust function call (legacy). [name:string, args:[node…]]",
      "type": "object",
      "required": ["NativeCall"],
      "properties": {
        "NativeCall": {
          "type": "array",
          "prefixItems": [
            { "type": "string" },
            { "type": "array", "items": { "$ref": "#" } }
          ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Read file contents to String. Requires --allow-read.",
      "type": "object",
      "required": ["FileRead"],
      "properties": { "FileRead": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Write String to file. Requires --allow-write. [path, content]",
      "type": "object",
      "required": ["FileWrite"],
      "properties": {
        "FileWrite": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Agent specialised read. Requires --allow-read.",
      "type": "object",
      "required": ["FSRead"],
      "properties": { "FSRead": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Agent specialised write. Requires --allow-write. [path, content]",
      "type": "object",
      "required": ["FSWrite"],
      "properties": {
        "FSWrite": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Async HTTP fetch. Requires --allow-net.",
      "type": "object",
      "required": ["Fetch"],
      "properties": {
        "Fetch": {
          "type": "object",
          "required": ["method", "url", "callback"],
          "properties": {
            "method":   { "type": "string", "enum": ["GET", "POST", "PUT", "DELETE"] },
            "url":      { "type": "string" },
            "callback": { "$ref": "#" }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Extract a value from a fetched payload by path. [source, path]",
      "type": "object",
      "required": ["Extract"],
      "properties": {
        "Extract": {
          "type": "object",
          "required": ["source", "path"],
          "properties": {
            "source": { "$ref": "#" },
            "path":   { "$ref": "#" }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Persistent KV store write.",
      "type": "object",
      "required": ["Store"],
      "properties": {
        "Store": {
          "type": "object",
          "required": ["key", "value"],
          "properties": {
            "key":   { "type": "string" },
            "value": { "$ref": "#" }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Persistent KV store read.",
      "type": "object",
      "required": ["Load"],
      "properties": {
        "Load": {
          "type": "object",
          "required": ["key"],
          "properties": { "key": { "type": "string" } },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Evaluate a JSON string natively into a RelType value.",
      "type": "object",
      "required": ["EvalJSONNative"],
      "properties": { "EvalJSONNative": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "egui window container. [id:string, title:node, children:Block]",
      "type": "object",
      "required": ["UIWindow"],
      "properties": {
        "UIWindow": {
          "type": "array",
          "prefixItems": [ { "type": "string" }, { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 3, "maxItems": 3
        }
      },
      "additionalProperties": false
    },
    {
      "description": "egui text label.",
      "type": "object",
      "required": ["UILabel"],
      "properties": { "UILabel": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "egui button. Evaluates to Bool(true) when clicked this frame.",
      "type": "object",
      "required": ["UIButton"],
      "properties": { "UIButton": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "egui single-line text field. State-binding: text = UITextInput(text). Reads from UI_TEXT_INPUT_BUFFER.",
      "type": "object",
      "required": ["UITextInput"],
      "properties": { "UITextInput": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Horizontal layout list of children.",
      "type": "object",
      "required": ["UIHBox"],
      "properties": {
        "UIHBox": { "type": "array", "items": { "$ref": "#" } }
      },
      "additionalProperties": false
    },
    {
      "description": "Vertical layout list of children.",
      "type": "object",
      "required": ["UIVBox"],
      "properties": {
        "UIVBox": { "type": "array", "items": { "$ref": "#" } }
      },
      "additionalProperties": false
    },
    {
      "description": "Horizontal layout (single body block).",
      "type": "object",
      "required": ["UIHorizontal"],
      "properties": { "UIHorizontal": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Full-canvas borderless panel.",
      "type": "object",
      "required": ["UIFullscreen"],
      "properties": { "UIFullscreen": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Grid layout. [columns:Int, id:string, body:Block]",
      "type": "object",
      "required": ["UIGrid"],
      "properties": {
        "UIGrid": {
          "type": "array",
          "prefixItems": [ { "type": "integer" }, { "type": "string" }, { "$ref": "#" } ],
          "minItems": 3, "maxItems": 3
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Scrollable container. [id:string, body:Block]",
      "type": "object",
      "required": ["UIScrollArea"],
      "properties": {
        "UIScrollArea": {
          "type": "array",
          "prefixItems": [ { "type": "string" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Fixed-size container.",
      "type": "object",
      "required": ["UIFixed"],
      "properties": {
        "UIFixed": {
          "type": "object",
          "required": ["width", "height", "body"],
          "properties": {
            "width":  { "$ref": "#" },
            "height": { "$ref": "#" },
            "body":   { "$ref": "#" }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Expand to fill parent container dimensions.",
      "type": "string",
      "enum": ["UIFillParent"]
    },
    {
      "description": "Global visual style. Arity 4 to 6 arguments allowed. Optional: btn_idle, btn_hover. [rounding, spacing, accent_rgba, fill_rgba, btn_idle?, btn_hover?]",
      "type": "object",
      "required": ["UISetStyle"],
      "properties": {
        "UISetStyle": {
          "type": "array",
          "items": { "$ref": "#" },
          "minItems": 4, "maxItems": 6
        }
      },
      "additionalProperties": false
    },
    {
      "description": "2D GPU rectangle. color is Array[R,G,B,A] floats 0.0–1.0.",
      "type": "object",
      "required": ["DrawRect"],
      "properties": {
        "DrawRect": {
          "type": "object",
          "required": ["x", "y", "width", "height", "color"],
          "properties": {
            "x":      { "$ref": "#" },
            "y":      { "$ref": "#" },
            "width":  { "$ref": "#" },
            "height": { "$ref": "#" },
            "color":  { "$ref": "#" }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Bootstrap WGPU window. [width:Int, height:Int, title:String]",
      "type": "object",
      "required": ["InitWindow"],
      "properties": {
        "InitWindow": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 3, "maxItems": 3
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Bootstrap WGPU context (unit node).",
      "type": "string",
      "enum": ["InitGraphics"]
    },
    {
      "description": "Load WGSL shader source string. Returns shader handle.",
      "type": "object",
      "required": ["LoadShader"],
      "properties": { "LoadShader": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Render a mesh with a shader. [shader_id, vertices, mvp_matrix]",
      "type": "object",
      "required": ["RenderMesh"],
      "properties": {
        "RenderMesh": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 3, "maxItems": 3
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Poll winit events and yield control. [body]",
      "type": "object",
      "required": ["PollEvents"],
      "properties": { "PollEvents": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Load OBJ/GLTF mesh from path.",
      "type": "object",
      "required": ["LoadMesh"],
      "properties": { "LoadMesh": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Load image texture from path.",
      "type": "object",
      "required": ["LoadTexture"],
      "properties": { "LoadTexture": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Load TrueType font from path.",
      "type": "object",
      "required": ["LoadFont"],
      "properties": { "LoadFont": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Draw text. [text, x, y, size, color_array]",
      "type": "object",
      "required": ["DrawText"],
      "properties": {
        "DrawText": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" }, { "$ref": "#" }, { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 5, "maxItems": 5
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Render full asset pipeline. [shader_id, mesh_id, texture_id, mvp_matrix]",
      "type": "object",
      "required": ["RenderAsset"],
      "properties": {
        "RenderAsset": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" }, { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 4, "maxItems": 4
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Perspective camera node.",
      "type": "object",
      "required": ["Camera3D"],
      "properties": {
        "Camera3D": {
          "type": "object",
          "required": ["pos_x","pos_y","pos_z","target_x","target_y","target_z","fov"],
          "properties": {
            "pos_x":    { "$ref": "#" }, "pos_y":    { "$ref": "#" }, "pos_z":    { "$ref": "#" },
            "target_x": { "$ref": "#" }, "target_y": { "$ref": "#" }, "target_z": { "$ref": "#" },
            "fov":      { "$ref": "#" }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "3D mesh node. primitive: 'cube'|'sphere'|'plane'.",
      "type": "object",
      "required": ["Mesh3D"],
      "properties": {
        "Mesh3D": {
          "type": "object",
          "required": ["primitive", "material"],
          "properties": {
            "primitive": { "$ref": "#" },
            "material":  { "$ref": "#" }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "PBR material definition.",
      "type": "object",
      "required": ["Material3D"],
      "properties": {
        "Material3D": {
          "type": "object",
          "required": ["r","g","b","a","metallic","roughness"],
          "properties": {
            "r": { "$ref": "#" }, "g": { "$ref": "#" }, "b": { "$ref": "#" }, "a": { "$ref": "#" },
            "metallic":   { "$ref": "#" },
            "roughness":  { "$ref": "#" },
            "texture_id": { "$ref": "#" }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Point light in world space.",
      "type": "object",
      "required": ["PointLight3D"],
      "properties": {
        "PointLight3D": {
          "type": "object",
          "required": ["x","y","z","r","g","b","intensity"],
          "properties": {
            "x": { "$ref": "#" }, "y": { "$ref": "#" }, "z": { "$ref": "#" },
            "r": { "$ref": "#" }, "g": { "$ref": "#" }, "b": { "$ref": "#" },
            "intensity": { "$ref": "#" }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Instanced 3D mesh with transform, PBR and color offset.",
      "type": "object",
      "required": ["MeshInstance3D"],
      "properties": {
        "MeshInstance3D": {
          "type": "object",
          "required": ["mesh_id","transform","color_offset","pbr"],
          "properties": {
            "mesh_id":      { "$ref": "#" },
            "transform":    { "$ref": "#" },
            "color_offset": { "$ref": "#" },
            "pbr":          { "$ref": "#" }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Gravity-aware FPS camera. fov in degrees.",
      "type": "object",
      "required": ["FPSCamera"],
      "properties": {
        "FPSCamera": {
          "type": "object",
          "required": ["fov"],
          "properties": { "fov": { "$ref": "#" } },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Capture / release mouse cursor.",
      "type": "object",
      "required": ["MouseGrab"],
      "properties": {
        "MouseGrab": {
          "type": "object",
          "required": ["enabled"],
          "properties": { "enabled": { "$ref": "#" } },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Screen-space weapon view model.",
      "type": "object",
      "required": ["WeaponViewModel"],
      "properties": {
        "WeaponViewModel": {
          "type": "object",
          "required": ["mesh","tex"],
          "properties": {
            "mesh": { "$ref": "#" },
            "tex":  { "$ref": "#" }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "WGPU 2D render canvas root.",
      "type": "object",
      "required": ["RenderCanvas"],
      "properties": {
        "RenderCanvas": {
          "type": "object",
          "required": ["body"],
          "properties": { "body": { "$ref": "#" } },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "2D transform node.",
      "type": "object",
      "required": ["Transform2D"],
      "properties": {
        "Transform2D": {
          "type": "object",
          "required": ["x","y","rotation","scale","body"],
          "properties": {
            "x":        { "$ref": "#" },
            "y":        { "$ref": "#" },
            "rotation": { "$ref": "#" },
            "scale":    { "$ref": "#" },
            "body":     { "$ref": "#" }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "2D textured sprite.",
      "type": "object",
      "required": ["Sprite2D"],
      "properties": {
        "Sprite2D": {
          "type": "object",
          "required": ["texture_id","transform"],
          "properties": {
            "texture_id": { "$ref": "#" },
            "transform":  { "$ref": "#" }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Activate 3D FPS camera (voxel engine). fov in degrees.",
      "type": "object",
      "required": ["InitCamera"],
      "properties": { "InitCamera": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Render voxel grid from position array.",
      "type": "object",
      "required": ["DrawVoxelGrid"],
      "properties": { "DrawVoxelGrid": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Load tiled texture atlas. [path, tile_size:Float]",
      "type": "object",
      "required": ["LoadTextureAtlas"],
      "properties": {
        "LoadTextureAtlas": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Set a voxel at position. [x, y, z, voxel_id]",
      "type": "object",
      "required": ["SetVoxel"],
      "properties": {
        "SetVoxel": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" }, { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 4, "maxItems": 4
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Toggle raycasting + mouse-block interaction.",
      "type": "object",
      "required": ["EnableInteraction"],
      "properties": { "EnableInteraction": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Toggle gravity and AABB collision physics.",
      "type": "object",
      "required": ["EnablePhysics"],
      "properties": { "EnablePhysics": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Register world AABB collision barrier.",
      "type": "object",
      "required": ["AddWorldAABB"],
      "properties": {
        "AddWorldAABB": {
          "type": "object",
          "required": ["min","max"],
          "properties": {
            "min": { "$ref": "#" },
            "max": { "$ref": "#" }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "AABB vs AABB collision check. Returns Bool.",
      "type": "object",
      "required": ["CheckCollision"],
      "properties": {
        "CheckCollision": {
          "type": "object",
          "required": ["a_min","a_max","b_min","b_max"],
          "properties": {
            "a_min": { "$ref": "#" }, "a_max": { "$ref": "#" },
            "b_min": { "$ref": "#" }, "b_max": { "$ref": "#" }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Load audio sample. [id:Int, path:String]",
      "type": "object",
      "required": ["LoadSample"],
      "properties": {
        "LoadSample": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 2, "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Play audio sample. [id, volume, pitch]",
      "type": "object",
      "required": ["PlaySample"],
      "properties": {
        "PlaySample": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 3, "maxItems": 3
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Play synthesised note. [channel, frequency, waveform]",
      "type": "object",
      "required": ["PlayNote"],
      "properties": {
        "PlayNote": {
          "type": "array",
          "prefixItems": [ { "$ref": "#" }, { "$ref": "#" }, { "$ref": "#" } ],
          "minItems": 3, "maxItems": 3
        }
      },
      "additionalProperties": false
    },
    {
      "description": "Stop synthesised note on channel.",
      "type": "object",
      "required": ["StopNote"],
      "properties": { "StopNote": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Play audio file from path.",
      "type": "object",
      "required": ["PlayAudioFile"],
      "properties": { "PlayAudioFile": { "$ref": "#" } },
      "additionalProperties": false
    },
    {
      "description": "Initialise CPAL audio engine (unit node).",
      "type": "string",
      "enum": ["InitAudio"]
    },
    {
      "description": "Transfer voxel state to mutable HashMap (unit node).",
      "type": "string",
      "enum": ["InitVoxelMap"]
    },
    {
      "description": "Engine uptime float in seconds (unit node).",
      "type": "string",
      "enum": ["Time"]
    },
    {
      "description": "Monotonic global time float (unit node).",
      "type": "string",
      "enum": ["GlobalTime"]
    },
    {
      "description": "Last keyboard character as String (unit node).",
      "type": "string",
      "enum": ["GetLastKeypress"]
    },
    {
      "description": "Perform a single raycasting query at screen centre (unit node).",
      "type": "string",
      "enum": ["RaycastSimple"]
    }
  ]
}

```

## Standard Native FFI Functions

```json
{
  "registry_version": "v1.1.0",
  "_comment": "Machine-readable registry of every native FFI function exposed by KnotenCore. All functions are invoked via ExternCall { module, function, args } or the equivalent stdlib wrapper. AI agents MUST only call functions listed here — hallucinated names will return ExecResult::Fault at runtime.",
  "modules": {
    "registry": "WGPU window management, textures, 3D rendering, input, timers, physics",
    "ui":       "egui immediate-mode GUI (legacy shim + UITextInput buffer)",
    "fs":       "Filesystem utilities, JSON parsing, array/object helpers",
    "net":      "HTTP networking (sandboxed)",
    "json":     "JSON serialisation helpers (alias layer over fs module)",
    "time":     "CPU throttling via std::thread::sleep"
  },
  "functions": [
    {
      "name": "registry_create_window",
      "module": "registry",
      "description": "Bootstraps a WGPU egui window and returns an opaque Handle. This is the preferred window creation path for interactive UI scripts.",
      "parameters": [
        { "name": "width",  "type": "Int",    "required": true, "description": "Window width in pixels"  },
        { "name": "height", "type": "Int",    "required": true, "description": "Window height in pixels" },
        { "name": "title",  "type": "String", "required": true, "description": "Window title bar string" }
      ],
      "returns": "Handle",
      "permissions": [],
      "errors": [],
      "example": {
        "ExternCall": {
          "module": "registry",
          "function": "registry_create_window",
          "args": [
            { "IntLiteral": 800 },
            { "IntLiteral": 600 },
            { "StringLiteral": "My App" }
          ]
        }
      }
    },
    {
      "name": "registry_window_update",
      "module": "registry",
      "description": "Pumps the OS event loop and redraws the WGPU frame. Returns Bool(true) while the window is open; false when the user closes it. Call once per loop iteration.",
      "parameters": [
        { "name": "window", "type": "Handle", "required": true, "description": "Handle from registry_create_window" }
      ],
      "returns": "Bool",
      "permissions": [],
      "errors": ["ERR_INVALID_HANDLE"],
      "example": {
        "Assign": ["open", {
          "ExternCall": {
            "module": "registry",
            "function": "registry_window_update",
            "args": [ { "Identifier": "win" } ]
          }
        }]
      }
    },
    {
      "name": "registry_window_close",
      "module": "registry",
      "description": "Destroys the window and releases its GPU resources from the ARC registry.",
      "parameters": [
        { "name": "window", "type": "Handle", "required": true, "description": "Handle from registry_create_window" }
      ],
      "returns": "Void",
      "permissions": [],
      "errors": ["ERR_INVALID_HANDLE"],
      "example": {
        "ExternCall": {
          "module": "registry",
          "function": "registry_window_close",
          "args": [ { "Identifier": "win" } ]
        }
      }
    },
    {
      "name": "registry_fill_color",
      "module": "registry",
      "description": "Clears the frame buffer to a solid RGB colour (0–255 per channel). Call at the start of each render loop before drawing primitives.",
      "parameters": [
        { "name": "window", "type": "Handle", "required": true  },
        { "name": "r",      "type": "Int",    "required": true, "description": "Red   0–255" },
        { "name": "g",      "type": "Int",    "required": true, "description": "Green 0–255" },
        { "name": "b",      "type": "Int",    "required": true, "description": "Blue  0–255" }
      ],
      "returns": "Void",
      "permissions": [],
      "errors": ["ERR_INVALID_HANDLE"],
      "example": {
        "ExternCall": {
          "module": "registry",
          "function": "registry_fill_color",
          "args": [
            { "Identifier": "win" },
            { "IntLiteral": 20 },
            { "IntLiteral": 25 },
            { "IntLiteral": 30 }
          ]
        }
      }
    },
    {
      "name": "registry_texture_load",
      "module": "registry",
      "description": "Loads an image file from disk into VRAM and returns a texture Handle. Path must be relative to the working directory.",
      "parameters": [
        { "name": "path", "type": "String", "required": true, "description": "Relative path to PNG/JPEG/BMP file" }
      ],
      "returns": "Handle",
      "permissions": ["--allow-read"],
      "errors": ["ERR_IO_PERMISSION", "ERR_FILE_NOT_FOUND", "ERR_PATH_ESCAPE"],
      "example": {
        "Assign": ["tex", {
          "ExternCall": {
            "module": "registry",
            "function": "registry_texture_load",
            "args": [ { "StringLiteral": "assets/textures/uv_checker.png" } ]
          }
        }]
      }
    },
    {
      "name": "registry_draw_quad_3d",
      "module": "registry",
      "description": "Renders a textured quad in 3D world space. Uses geometry caching — vertices computed once per unique scale.",
      "parameters": [
        { "name": "window",   "type": "Handle", "required": true },
        { "name": "texture",  "type": "Handle", "required": true },
        { "name": "x",        "type": "Float",  "required": true },
        { "name": "y",        "type": "Float",  "required": true },
        { "name": "z",        "type": "Float",  "required": true },
        { "name": "scale_x",  "type": "Float",  "required": true },
        { "name": "scale_y",  "type": "Float",  "required": true }
      ],
      "returns": "Void",
      "permissions": [],
      "errors": ["ERR_INVALID_HANDLE"],
      "example": {
        "ExternCall": {
          "module": "registry",
          "function": "registry_draw_quad_3d",
          "args": [
            { "Identifier": "win" }, { "Identifier": "tex" },
            { "FloatLiteral": 0.0 }, { "FloatLiteral": 0.0 }, { "FloatLiteral": 0.0 },
            { "FloatLiteral": 1.0 }, { "FloatLiteral": 1.0 }
          ]
        }
      }
    },
    {
      "name": "registry_draw_sphere",
      "module": "registry",
      "description": "Renders a UV-sphere in 3D world space. Geometry is cached per (radius, rings, sectors) combination.",
      "parameters": [
        { "name": "window",  "type": "Handle", "required": true },
        { "name": "texture", "type": "Handle", "required": true },
        { "name": "radius",  "type": "Float",  "required": true },
        { "name": "rings",   "type": "Int",    "required": true, "description": "Latitude subdivisions (min 4)" },
        { "name": "sectors", "type": "Int",    "required": true, "description": "Longitude subdivisions (min 4)" },
        { "name": "x",       "type": "Float",  "required": true },
        { "name": "y",       "type": "Float",  "required": true },
        { "name": "z",       "type": "Float",  "required": true }
      ],
      "returns": "Void",
      "permissions": [],
      "errors": ["ERR_INVALID_HANDLE"],
      "example": {
        "ExternCall": {
          "module": "registry",
          "function": "registry_draw_sphere",
          "args": [
            { "Identifier": "win" }, { "Identifier": "tex" },
            { "FloatLiteral": 1.0 }, { "IntLiteral": 16 }, { "IntLiteral": 16 },
            { "FloatLiteral": 0.0 }, { "FloatLiteral": 0.0 }, { "FloatLiteral": 0.0 }
          ]
        }
      }
    },
    {
      "name": "registry_draw_cube",
      "module": "registry",
      "description": "Renders a textured box in 3D world space.",
      "parameters": [
        { "name": "window",  "type": "Handle", "required": true },
        { "name": "texture", "type": "Handle", "required": true },
        { "name": "width",   "type": "Float",  "required": true },
        { "name": "height",  "type": "Float",  "required": true },
        { "name": "depth",   "type": "Float",  "required": true },
        { "name": "x",       "type": "Float",  "required": true },
        { "name": "y",       "type": "Float",  "required": true },
        { "name": "z",       "type": "Float",  "required": true }
      ],
      "returns": "Void",
      "permissions": [],
      "errors": ["ERR_INVALID_HANDLE"],
      "example": {
        "ExternCall": {
          "module": "registry",
          "function": "registry_draw_cube",
          "args": [
            { "Identifier": "win" }, { "Identifier": "tex" },
            { "FloatLiteral": 1.0 }, { "FloatLiteral": 1.0 }, { "FloatLiteral": 1.0 },
            { "FloatLiteral": 0.0 }, { "FloatLiteral": 0.0 }, { "FloatLiteral": 0.0 }
          ]
        }
      }
    },
    {
      "name": "registry_draw_cylinder",
      "module": "registry",
      "description": "Renders a textured cylinder in 3D world space.",
      "parameters": [
        { "name": "window",  "type": "Handle", "required": true },
        { "name": "texture", "type": "Handle", "required": true },
        { "name": "radius",  "type": "Float",  "required": true },
        { "name": "height",  "type": "Float",  "required": true },
        { "name": "segments","type": "Int",    "required": true },
        { "name": "x",       "type": "Float",  "required": true },
        { "name": "y",       "type": "Float",  "required": true },
        { "name": "z",       "type": "Float",  "required": true }
      ],
      "returns": "Void",
      "permissions": [],
      "errors": ["ERR_INVALID_HANDLE"],
      "example": {
        "ExternCall": {
          "module": "registry",
          "function": "registry_draw_cylinder",
          "args": [
            { "Identifier": "win" }, { "Identifier": "tex" },
            { "FloatLiteral": 1.0 }, { "FloatLiteral": 2.0 }, { "IntLiteral": 16 },
            { "FloatLiteral": 0.0 }, { "FloatLiteral": 0.0 }, { "FloatLiteral": 0.0 }
          ]
        }
      }
    },
    {
      "name": "registry_draw_entity",
      "module": "registry",
      "description": "Renders an entity in 3D world space.",
      "parameters": [
        { "name": "window", "type": "Handle", "required": true },
        { "name": "x",      "type": "Float",  "required": true },
        { "name": "y",      "type": "Float",  "required": true }
      ],
      "returns": "Void",
      "permissions": [],
      "errors": ["ERR_INVALID_HANDLE"],
      "example": {
        "ExternCall": {
          "module": "registry",
          "function": "registry_draw_entity",
          "args": [
            { "Identifier": "win" },
            { "FloatLiteral": 0.0 }, { "FloatLiteral": 0.0 }
          ]
        }
      }
    },
    {
      "name": "registry_file_create",
      "module": "registry",
      "description": "Creates a new file at the given path and returns a writable file Handle. Path must be within the working directory.",
      "parameters": [
        { "name": "path", "type": "String", "required": true, "description": "Relative file path to create" }
      ],
      "returns": "Handle",
      "permissions": ["--allow-write"],
      "errors": ["ERR_IO_PERMISSION", "ERR_PATH_ESCAPE"],
      "example": {
        "Assign": ["fh", {
          "ExternCall": {
            "module": "registry",
            "function": "registry_file_create",
            "args": [ { "StringLiteral": "output/result.txt" } ]
          }
        }]
      }
    },
    {
      "name": "registry_file_write",
      "module": "registry",
      "description": "Appends or writes content to an open file Handle.",
      "parameters": [
        { "name": "handle",  "type": "Handle", "required": true },
        { "name": "content", "type": "String", "required": true }
      ],
      "returns": "Void",
      "permissions": ["--allow-write"],
      "errors": ["ERR_IO_PERMISSION", "ERR_INVALID_HANDLE"],
      "example": {
        "ExternCall": {
          "module": "registry",
          "function": "registry_file_write",
          "args": [
            { "Identifier": "fh" },
            { "StringLiteral": "Hello, World!" }
          ]
        }
      }
    },
    {
      "name": "registry_read_file",
      "module": "registry",
      "description": "Reads the entire contents of a file and returns a String. Path is canonicalized and must remain within the working directory.",
      "parameters": [
        { "name": "path", "type": "String", "required": true }
      ],
      "returns": "String",
      "permissions": ["--allow-read"],
      "errors": ["ERR_IO_PERMISSION", "ERR_FILE_NOT_FOUND", "ERR_PATH_ESCAPE"],
      "example": {
        "Assign": ["content", {
          "ExternCall": {
            "module": "registry",
            "function": "registry_read_file",
            "args": [ { "StringLiteral": "data/config.json" } ]
          }
        }]
      }
    },
    {
      "name": "registry_write_file",
      "module": "registry",
      "description": "Writes a String to a file, creating it if it does not exist. Returns Bool(true) on success.",
      "parameters": [
        { "name": "path",    "type": "String", "required": true },
        { "name": "content", "type": "String", "required": true }
      ],
      "returns": "Bool",
      "permissions": ["--allow-write"],
      "errors": ["ERR_IO_PERMISSION", "ERR_PATH_ESCAPE"],
      "example": {
        "ExternCall": {
          "module": "registry",
          "function": "registry_write_file",
          "args": [
            { "StringLiteral": "output/result.txt" },
            { "StringLiteral": "42" }
          ]
        }
      }
    },
    {
      "name": "registry_is_key_pressed",
      "module": "registry",
      "description": "Polls the lock-free AtomicBool key table at O(1). Returns Bool(true) if the physical key is currently held. Supported keys: W A S D SPACE UP DOWN LEFT RIGHT.",
      "parameters": [
        { "name": "key", "type": "String", "required": true, "description": "Uppercase key name: 'W', 'A', 'S', 'D', 'SPACE', 'UP', 'DOWN', 'LEFT', 'RIGHT'" }
      ],
      "returns": "Bool",
      "permissions": [],
      "errors": [],
      "example": {
        "If": [
          { "ExternCall": { "module": "registry", "function": "registry_is_key_pressed", "args": [ { "StringLiteral": "W" } ] } },
          { "Assign": ["y", { "Add": [ { "Identifier": "y" }, { "FloatLiteral": 0.1 } ] }] },
          null
        ]
      }
    },
    {
      "name": "registry_get_mouse_delta_x",
      "module": "registry",
      "description": "Returns the horizontal mouse movement delta (Float) since the last frame. Used for FPS camera look.",
      "parameters": [],
      "returns": "Float",
      "permissions": [],
      "errors": [],
      "example": {
        "Assign": ["dx", {
          "ExternCall": { "module": "registry", "function": "registry_get_mouse_delta_x", "args": [] }
        }]
      }
    },
    {
      "name": "registry_get_mouse_delta_y",
      "module": "registry",
      "description": "Returns the vertical mouse movement delta (Float) since the last frame.",
      "parameters": [],
      "returns": "Float",
      "permissions": [],
      "errors": [],
      "example": {
        "Assign": ["dy", {
          "ExternCall": { "module": "registry", "function": "registry_get_mouse_delta_y", "args": [] }
        }]
      }
    },
    {
      "name": "registry_get_last_char",
      "module": "registry",
      "description": "Returns the Unicode codepoint (Int) of the last character typed by the user, or 0 if none.",
      "parameters": [],
      "returns": "Int",
      "permissions": [],
      "errors": [],
      "example": {
        "Assign": ["ch", {
          "ExternCall": { "module": "registry", "function": "registry_get_last_char", "args": [] }
        }]
      }
    },
    {
      "name": "registry_now",
      "module": "registry",
      "description": "Captures the current monotonic timestamp and returns a timer Handle. Use with registry_elapsed_ms to measure durations.",
      "parameters": [],
      "returns": "Handle",
      "permissions": [],
      "errors": [],
      "example": {
        "Assign": ["t0", {
          "ExternCall": { "module": "registry", "function": "registry_now", "args": [] }
        }]
      }
    },
    {
      "name": "registry_elapsed_ms",
      "module": "registry",
      "description": "Returns the elapsed milliseconds (Int) since the timestamp captured by registry_now.",
      "parameters": [
        { "name": "timer", "type": "Handle", "required": true, "description": "Handle from registry_now" }
      ],
      "returns": "Int",
      "permissions": [],
      "errors": ["ERR_INVALID_HANDLE"],
      "example": {
        "Assign": ["ms", {
          "ExternCall": {
            "module": "registry",
            "function": "registry_elapsed_ms",
            "args": [ { "Identifier": "t0" } ]
          }
        }]
      }
    },
    {
      "name": "registry_set_camera",
      "module": "registry",
      "description": "Configures the global perspective camera with field-of-view and eye position. Affects all subsequent draw calls.",
      "parameters": [
        { "name": "fov", "type": "Float", "required": true, "description": "Field of view in degrees" },
        { "name": "x",   "type": "Float", "required": true },
        { "name": "y",   "type": "Float", "required": true },
        { "name": "z",   "type": "Float", "required": true }
      ],
      "returns": "Void",
      "permissions": [],
      "errors": [],
      "example": {
        "ExternCall": {
          "module": "registry",
          "function": "registry_set_camera",
          "args": [
            { "FloatLiteral": 60.0 },
            { "FloatLiteral": 0.0 }, { "FloatLiteral": 1.0 }, { "FloatLiteral": -5.0 }
          ]
        }
      }
    },
    {
      "name": "registry_dump",
      "module": "registry",
      "description": "Returns the total number of live handles in the ARC registry (Int). Useful for diagnosing handle leaks.",
      "parameters": [],
      "returns": "Int",
      "permissions": [],
      "errors": [],
      "example": {
        "Assign": ["n", {
          "ExternCall": { "module": "registry", "function": "registry_dump", "args": [] }
        }]
      }
    },
    {
      "name": "net_fetch",
      "module": "net",
      "description": "Sends a synchronous HTTP GET request and returns the response body as a String. Requires --allow-net.",
      "parameters": [
        { "name": "url", "type": "String", "required": true }
      ],
      "returns": "String",
      "permissions": ["--allow-net"],
      "errors": ["ERR_NET_PERMISSION", "ERR_HTTP_FAILED"],
      "example": {
        "Assign": ["body", {
          "ExternCall": {
            "module": "net",
            "function": "net_fetch",
            "args": [ { "StringLiteral": "https://api.github.com/repos/holgerbaer-bl/KnotenCore" } ]
          }
        }]
      }
    },
    {
      "name": "fs_read_file",
      "module": "fs",
      "description": "Reads a file and returns its full content as a String. Path must be relative and within the working directory.",
      "parameters": [
        { "name": "path", "type": "String", "required": true }
      ],
      "returns": "String",
      "permissions": ["--allow-read"],
      "errors": ["ERR_IO_PERMISSION", "ERR_FILE_NOT_FOUND"],
      "example": {
        "Assign": ["data", {
          "ExternCall": {
            "module": "fs",
            "function": "fs_read_file",
            "args": [ { "StringLiteral": "data/input.txt" } ]
          }
        }]
      }
    },
    {
      "name": "fs_parse_json",
      "module": "fs",
      "description": "Parses a JSON string into a native KnotenCore value (Object, Array, Int, Float, Bool, or String).",
      "parameters": [
        { "name": "json", "type": "String", "required": true }
      ],
      "returns": "Any",
      "permissions": [],
      "errors": ["ERR_JSON_PARSE"],
      "example": {
        "Assign": ["parsed", {
          "ExternCall": {
            "module": "fs",
            "function": "fs_parse_json",
            "args": [ { "Identifier": "raw_json" } ]
          }
        }]
      }
    },
    {
      "name": "json_parse",
      "module": "json",
      "description": "Alias for fs_parse_json. Parses a JSON string into a native KnotenCore value.",
      "parameters": [
        { "name": "payload", "type": "String", "required": true }
      ],
      "returns": "Any",
      "permissions": [],
      "errors": ["ERR_JSON_PARSE"],
      "example": {
        "Assign": ["obj", {
          "ExternCall": {
            "module": "json",
            "function": "json_parse",
            "args": [ { "Identifier": "response_body" } ]
          }
        }]
      }
    },
    {
      "name": "json_stringify",
      "module": "json",
      "description": "Serialises any KnotenCore value into a JSON string.",
      "parameters": [
        { "name": "value", "type": "Any", "required": true }
      ],
      "returns": "String",
      "permissions": [],
      "errors": [],
      "example": {
        "Assign": ["s", {
          "ExternCall": {
            "module": "json",
            "function": "json_stringify",
            "args": [ { "Identifier": "my_object" } ]
          }
        }]
      }
    },
    {
      "name": "array_length",
      "module": "fs",
      "description": "Returns the length of an Array as Int.",
      "parameters": [
        { "name": "array", "type": "Array", "required": true }
      ],
      "returns": "Int",
      "permissions": [],
      "errors": [],
      "example": {
        "Assign": ["n", {
          "ExternCall": {
            "module": "fs",
            "function": "array_length",
            "args": [ { "Identifier": "items" } ]
          }
        }]
      }
    },
    {
      "name": "array_get",
      "module": "fs",
      "description": "Returns the element at index from an Array. Returns Void if index is out of bounds.",
      "parameters": [
        { "name": "array", "type": "Array", "required": true },
        { "name": "index", "type": "Int",   "required": true }
      ],
      "returns": "Any",
      "permissions": [],
      "errors": [],
      "example": {
        "Assign": ["item", {
          "ExternCall": {
            "module": "fs",
            "function": "array_get",
            "args": [ { "Identifier": "items" }, { "IntLiteral": 0 } ]
          }
        }]
      }
    },
    {
      "name": "obj_get",
      "module": "fs",
      "description": "Returns the value for a key from an Object map. Returns Void if key is absent.",
      "parameters": [
        { "name": "object", "type": "Object", "required": true },
        { "name": "key",    "type": "String", "required": true }
      ],
      "returns": "Any",
      "permissions": [],
      "errors": [],
      "example": {
        "Assign": ["name", {
          "ExternCall": {
            "module": "fs",
            "function": "obj_get",
            "args": [ { "Identifier": "data" }, { "StringLiteral": "name" } ]
          }
        }]
      }
    },
    {
      "name": "obj_set",
      "module": "fs",
      "description": "Returns a new Object with the given key set to the given value (immutable update pattern).",
      "parameters": [
        { "name": "object", "type": "Object", "required": true },
        { "name": "key",    "type": "String", "required": true },
        { "name": "value",  "type": "Any",    "required": true }
      ],
      "returns": "Object",
      "permissions": [],
      "errors": [],
      "example": {
        "Assign": ["data2", {
          "ExternCall": {
            "module": "fs",
            "function": "obj_set",
            "args": [
              { "Identifier": "data" },
              { "StringLiteral": "score" },
              { "IntLiteral": 42 }
            ]
          }
        }]
      }
    },
    {
      "name": "obj_has_key",
      "module": "fs",
      "description": "Returns Bool(true) if the Object contains the given key.",
      "parameters": [
        { "name": "object", "type": "Object", "required": true },
        { "name": "key",    "type": "String", "required": true }
      ],
      "returns": "Bool",
      "permissions": [],
      "errors": [],
      "example": {
        "ExternCall": {
          "module": "fs",
          "function": "obj_has_key",
          "args": [ { "Identifier": "data" }, { "StringLiteral": "name" } ]
        }
      }
    },
    {
      "name": "time_sleep_ms",
      "module": "time",
      "description": "Blocks the executor thread for at least the given number of milliseconds. Use for frame pacing (~16ms = 60 FPS cap). No-op if ms <= 0.",
      "parameters": [
        { "name": "milliseconds", "type": "Int", "required": true }
      ],
      "returns": "Void",
      "permissions": [],
      "errors": [],
      "example": {
        "ExternCall": {
          "module": "time",
          "function": "time_sleep_ms",
          "args": [ { "IntLiteral": 16 } ]
        }
      }
    },
    {
      "name": "ui_text_input_get",
      "module": "ui",
      "description": "Reads the current value of the global UI_TEXT_INPUT_BUFFER. Returns String. Part of the UITextInput state-binding pattern.",
      "parameters": [],
      "returns": "String",
      "permissions": [],
      "errors": [],
      "example": {
        "Assign": ["text", {
          "ExternCall": { "module": "ui", "function": "ui_text_input_get", "args": [] }
        }]
      }
    },
    {
      "name": "ui_text_input_set",
      "module": "ui",
      "description": "Overwrites the global UI_TEXT_INPUT_BUFFER. Call to pre-seed a text field before the render loop starts.",
      "parameters": [
        { "name": "value", "type": "String", "required": true }
      ],
      "returns": "Void",
      "permissions": [],
      "errors": [],
      "example": {
        "ExternCall": {
          "module": "ui",
          "function": "ui_text_input_set",
          "args": [ { "StringLiteral": "Initial text..." } ]
        }
      }
    }
  ]
}

```

## Error Code Output Matrix

```json
{
  "catalog_version": "v1.0.0",
  "description": "Fehlerkatalog für strukturierte Self-Healing-Loops durch KI-Agenten.",
  "errors": [
    {
      "code": "ERR_UNKNOWN_NODE",
      "category": "Syntax",
      "message_pattern": "Unrecognized node type: {}",
      "agent_hint": "Check node_types.json! You probably emitted a hallucinated or deprecated node type instead of a valid AST node."
    },
    {
      "code": "ERR_ARITY_MISMATCH",
      "category": "Syntax",
      "message_pattern": "Node {} expects {} arguments but got {}",
      "agent_hint": "Check the exact array structure for this node in node_types.json or nod_grammar.ebnf. Some parameters must be present, some can be optional."
    },
    {
      "code": "ERR_INVALID_HANDLE",
      "category": "Runtime",
      "message_pattern": "Invalid or expired Handle provided to {}",
      "agent_hint": "Ensure the handle was properly created (e.g., registry_create_window) before passing it to this function."
    },
    {
      "code": "ERR_IO_PERMISSION",
      "category": "Security",
      "message_pattern": "I/O operation blocked: Requires --allow-{}",
      "agent_hint": "You attempted an external file or system operation without the correct permission flag. Ask the user to restart the agent/executor with --allow-read or --allow-write."
    },
    {
      "code": "ERR_NET_PERMISSION",
      "category": "Security",
      "message_pattern": "Network operation blocked: Requires --allow-net",
      "agent_hint": "You attempted an external network request without the correct permission flag. Ask the user to restart the agent/executor with --allow-net."
    },
    {
      "code": "ERR_JSON_PARSE",
      "category": "Runtime",
      "message_pattern": "Failed to parse JSON string in {}",
      "agent_hint": "The provided string cannot be parsed as valid JSON. Check string escaping and format."
    }
  ]
}

```

## Antipatterns & Pitfalls

```javascript
// ================================================================
// FILE:    docs/LANGUAGE_REFERENCE/examples/99_antipatterns.nod
// PURPOSE: Explicit DO / DON'T guide for AI code agents.
//          Every block below shows a WRONG pattern followed by
//          the correct KnotenCore equivalent.
//          This file is NOT executable — it is a JSON comment file
//          for agent training and schema validation testing.
// ================================================================

// ----------------------------------------------------------------
// ANTI-PATTERN 1: Wrong node name for variable assignment
// ----------------------------------------------------------------

// ❌ WRONG — "Let" does not exist in KnotenCore. Rust / JS syntax
//            is not valid in .nod files.
// { "Let":  ["x", 42] }
// { "let":  ["x", {"IntLiteral": 42}] }
// { "Var":  ["x", {"IntLiteral": 42}] }
// { "Const":["x", {"IntLiteral": 42}] }

// ✅ CORRECT — Variable assignment always uses the "Assign" node.
//              The name is a plain JSON string; the value must be
//              a fully wrapped node.
// { "Assign": ["x", { "IntLiteral": 42 }] }


// ----------------------------------------------------------------
// ANTI-PATTERN 2: Bare scalar values instead of literal nodes
// ----------------------------------------------------------------

// ❌ WRONG — Bare strings, numbers, and booleans are not valid
//            as node values. The engine expects a node object.
// { "Assign": ["name", "Hello"] }
// { "Assign": ["score", 100] }
// { "Assign": ["active", true] }

// ✅ CORRECT — Every value must be wrapped in its literal node.
// { "Assign": ["name",   { "StringLiteral": "Hello" }] }
// { "Assign": ["score",  { "IntLiteral": 100 }] }
// { "Assign": ["active", { "BoolLiteral": true }] }


// ----------------------------------------------------------------
// ANTI-PATTERN 3: Hallucinated function names
// ----------------------------------------------------------------

// ❌ WRONG — These function names do not exist in the engine.
//            Calling them returns ExecResult::Fault at runtime.
// { "Call": ["file_open",    [{ "StringLiteral": "test.txt" }]] }
// { "Call": ["readFile",     [{ "StringLiteral": "test.txt" }]] }
// { "Call": ["open_window",  [{ "IntLiteral": 800 }, { "IntLiteral": 600 }]] }
// { "Call": ["createWindow", [{ "IntLiteral": 800 }, { "IntLiteral": 600 }]] }
// { "Call": ["http_get",     [{ "StringLiteral": "https://example.com" }]] }
// { "Call": ["sleep",        [{ "IntLiteral": 16 }]] }

// ✅ CORRECT — Only call functions listed in native_functions.json.
//              Use ExternCall with explicit module + function fields.
// { "ExternCall": { "module": "registry", "function": "registry_file_create",    "args": [{ "StringLiteral": "out.txt" }] } }
// { "ExternCall": { "module": "registry", "function": "registry_read_file",      "args": [{ "StringLiteral": "data.txt" }] } }
// { "ExternCall": { "module": "registry", "function": "registry_create_window",  "args": [{ "IntLiteral": 800 }, { "IntLiteral": 600 }, { "StringLiteral": "App" }] } }
// { "ExternCall": { "module": "net",      "function": "net_fetch",               "args": [{ "StringLiteral": "https://example.com" }] } }
// { "ExternCall": { "module": "time",     "function": "time_sleep_ms",           "args": [{ "IntLiteral": 16 }] } }


// ----------------------------------------------------------------
// ANTI-PATTERN 4: Wrong outer structure for ExternCall
// ----------------------------------------------------------------

// ❌ WRONG — ExternCall payload is a nested object, not an array.
//            Invented keys ("fn", "arguments") are rejected by
//            additionalProperties:false in the JSON Schema.
// { "ExternCall": ["registry", "registry_create_window", [800, 600, "App"]] }
// { "ExternCall": { "fn": "registry_create_window", "arguments": [] } }

// ✅ CORRECT — ExternCall always has exactly: module, function, args.
// {
//   "ExternCall": {
//     "module":   "registry",
//     "function": "registry_create_window",
//     "args": [
//       { "IntLiteral": 800 },
//       { "IntLiteral": 600 },
//       { "StringLiteral": "My App" }
//     ]
//   }
// }


// ----------------------------------------------------------------
// ANTI-PATTERN 5: Inlining raw object literals where a node is expected
// ----------------------------------------------------------------

// ❌ WRONG — ObjectLiteral must be wrapped in the "ObjectLiteral" key.
//            A raw JSON object at node position is not valid.
// { "Assign": ["player", { "x": 0.0, "y": 0.0 }] }

// ✅ CORRECT — Use the ObjectLiteral node; values must be nodes too.
// {
//   "Assign": ["player", {
//     "ObjectLiteral": {
//       "x": { "FloatLiteral": 0.0 },
//       "y": { "FloatLiteral": 0.0 }
//     }
//   }]
// }


// ----------------------------------------------------------------
// ANTI-PATTERN 6: Missing else-branch null in If tuples
// ----------------------------------------------------------------

// ❌ WRONG — If with only 2 elements but where a 3rd was intended.
//            Structural ambiguity. Always be explicit.
// { "If": [{ "BoolLiteral": true }, { "Print": { "StringLiteral": "yes" } }] }

// ✅ CORRECT — Omit the 3rd element (maxItems:3) only if truly
//              no else branch is needed. 2-element arrays are valid.
// { "If": [
//     { "Identifier": "is_running" },
//     { "Print": { "StringLiteral": "still going" } }
// ]}


// ----------------------------------------------------------------
// ANTI-PATTERN 7: Arithmetic directly on Identifier without node wrapping
// ----------------------------------------------------------------

// ❌ WRONG — Cannot embed bare identifiers or numbers in Add.
// { "Add": ["x", 1] }

// ✅ CORRECT — Every argument to math nodes must be a node object.
// { "Add": [{ "Identifier": "x" }, { "IntLiteral": 1 }] }


// ----------------------------------------------------------------
// ANTI-PATTERN 8: Using UITextInput without state-binding assignment
// ----------------------------------------------------------------

// ❌ WRONG — Calling UITextInput and discarding the return value
//            means the script variable is never updated from the
//            egui buffer. The text field appears to do nothing.
// { "UITextInput": { "Identifier": "text" } }

// ✅ CORRECT — ALWAYS assign the return value back to the variable.
//              This is the idiomatic state-binding pattern.
// { "Assign": ["text", { "UITextInput": { "Identifier": "text" } }] }


// ----------------------------------------------------------------
// ANTI-PATTERN 9: Using I/O without permission flags
// ----------------------------------------------------------------

// ❌ WRONG — Running a script that calls registry_read_file without
//            passing --allow-read returns ERR_IO_PERMISSION Fault.
// cargo run --bin run_knc -- my_script.nod

// ✅ CORRECT — Always pass the required flag.
// cargo run --bin run_knc -- my_script.nod --allow-read
// cargo run --bin run_knc -- my_script.nod --allow-read --allow-write
// cargo run --bin run_knc -- my_script.nod --allow-net


// ----------------------------------------------------------------
// ANTI-PATTERN 10: Invented Return type in ExternCall
// ----------------------------------------------------------------

// ❌ WRONG — ExternCall has no "returns" field in the AST node.
//            Return types are documented in native_functions.json
//            for reference only; they are NOT part of the .nod syntax.
// {
//   "ExternCall": {
//     "module": "registry", "function": "registry_now",
//     "args": [], "returns": "Handle"
//   }
// }

// ✅ CORRECT — ExternCall has exactly three keys: module, function, args.
// { "ExternCall": { "module": "registry", "function": "registry_now", "args": [] } }

```
