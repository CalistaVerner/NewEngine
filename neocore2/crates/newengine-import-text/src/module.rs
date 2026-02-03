#![forbid(unsafe_op_in_unsafe_fn)]

use abi_stable::sabi_trait::TD_Opaque;
use abi_stable::std_types::{RResult, RString, RVec};
use abi_stable::StableAbi;

use newengine_plugin_api::{
    Blob, HostApiV1, MethodName, PluginInfo, PluginModule, ServiceV1, ServiceV1Dyn, ServiceV1_TO,
};

use serde_json::json;

/* =============================================================================================
   Wire helpers: [u32 meta_len_le][meta_json utf8][payload]
   ============================================================================================= */

#[inline]
fn pack(meta_json: &str, payload: &[u8]) -> RVec<u8> {
    let meta = meta_json.as_bytes();
    let meta_len: u32 = meta.len().min(u32::MAX as usize) as u32;

    let mut out = Vec::with_capacity(4 + meta.len() + payload.len());
    out.extend_from_slice(&meta_len.to_le_bytes());
    out.extend_from_slice(meta);
    out.extend_from_slice(payload);
    RVec::from(out)
}

#[inline]
fn ok_blob(v: RVec<u8>) -> RResult<RVec<u8>, RString> {
    RResult::ROk(v)
}

#[inline]
fn err(msg: impl Into<String>) -> RResult<RVec<u8>, RString> {
    RResult::RErr(RString::from(msg.into()))
}

#[inline]
fn strip_utf8_bom_bytes(bytes: &[u8]) -> &[u8] {
    // UTF-8 BOM: EF BB BF
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        &bytes[3..]
    } else {
        bytes
    }
}

#[inline]
fn meta(schema: &str, container: &str, byte_len: usize) -> String {
    json!({
        "schema": schema,
        "container": container,
        "encoding": "utf-8",
        "byte_len": byte_len as u64
    })
        .to_string()
}

/* =============================================================================================
   Import implementations
   ============================================================================================= */

fn import_txt(bytes: &[u8], container: &str, schema: &str) -> RResult<RVec<u8>, RString> {
    let payload = strip_utf8_bom_bytes(bytes);

    // Validate UTF-8 early: contract says payload is UTF-8.
    if let Err(e) = std::str::from_utf8(payload) {
        return err(format!("textimporter: utf8: {e}"));
    }

    let meta_json = meta(schema, container, payload.len());
    ok_blob(pack(&meta_json, payload))
}

fn import_json(bytes: &[u8]) -> RResult<RVec<u8>, RString> {
    let payload = strip_utf8_bom_bytes(bytes);

    // Validate UTF-8 + JSON.
    let s = match std::str::from_utf8(payload) {
        Ok(v) => v,
        Err(e) => return err(format!("textimporter: utf8: {e}")),
    };

    if let Err(e) = serde_json::from_str::<serde_json::Value>(s) {
        return err(format!("textimporter: json parse: {e}"));
    }

    let meta_json = meta("kalitech.text.meta.v1", "json", payload.len());
    ok_blob(pack(&meta_json, payload))
}

fn import_xml(bytes: &[u8]) -> RResult<RVec<u8>, RString> {
    let payload = strip_utf8_bom_bytes(bytes);

    // Validate UTF-8 + XML well-formedness.
    let s = match std::str::from_utf8(payload) {
        Ok(v) => v,
        Err(e) => return err(format!("textimporter: utf8: {e}")),
    };

    let mut r = quick_xml::Reader::from_reader(s.as_bytes());
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return err(format!("textimporter: xml parse: {e}")),
        }
        buf.clear();
    }

    let meta_json = meta("kalitech.text.meta.v1", "xml", payload.len());
    ok_blob(pack(&meta_json, payload))
}

/* =============================================================================================
   Service definitions
   ============================================================================================= */

#[derive(StableAbi)]
#[repr(C)]
struct TextJsonService;

impl ServiceV1 for TextJsonService {
    fn id(&self) -> RString {
        RString::from("kalitech.import.json.v1")
    }

    fn describe(&self) -> RString {
        RString::from(
            r#"{
  "id":"kalitech.import.json.v1",
  "kind":"asset_importer",
  "asset_importer":{
    "extensions":["json"],
    "output_type_id":"kalitech.asset.text",
    "format":"json",
    "method":"import_json_v1",
    "wire":"u32_meta_len_le + meta_utf8 + payload_utf8"
  },
  "meta_schema":"kalitech.text.meta.v1"
}"#,
        )
    }

    fn call(&self, method: MethodName, payload: Blob) -> RResult<Blob, RString> {
        match method.as_str() {
            "import_json_v1" => import_json(&payload.into_vec()).map(|v| v),
            _ => err(format!("textimporter(json): unknown method '{method}'")),
        }
    }
}

#[derive(StableAbi)]
#[repr(C)]
struct TextXmlService;

impl ServiceV1 for TextXmlService {
    fn id(&self) -> RString {
        RString::from("kalitech.import.xml.v1")
    }

    fn describe(&self) -> RString {
        RString::from(
            r#"{
  "id":"kalitech.import.xml.v1",
  "kind":"asset_importer",
  "asset_importer":{
    "extensions":["xml"],
    "output_type_id":"kalitech.asset.text",
    "format":"xml",
    "method":"import_xml_v1",
    "wire":"u32_meta_len_le + meta_utf8 + payload_utf8"
  },
  "meta_schema":"kalitech.text.meta.v1"
}"#,
        )
    }

    fn call(&self, method: MethodName, payload: Blob) -> RResult<Blob, RString> {
        match method.as_str() {
            "import_xml_v1" => import_xml(&payload.into_vec()).map(|v| v),
            _ => err(format!("textimporter(xml): unknown method '{method}'")),
        }
    }
}

#[derive(StableAbi)]
#[repr(C)]
struct TextHtmlService;

impl ServiceV1 for TextHtmlService {
    fn id(&self) -> RString {
        RString::from("kalitech.import.html.v1")
    }

    fn describe(&self) -> RString {
        RString::from(
            r#"{
  "id":"kalitech.import.html.v1",
  "kind":"asset_importer",
  "asset_importer":{
    "extensions":["html","htm"],
    "output_type_id":"kalitech.asset.text",
    "format":"html",
    "method":"import_html_v1",
    "wire":"u32_meta_len_le + meta_utf8 + payload_utf8"
  },
  "meta_schema":"kalitech.text.meta.v1"
}"#,
        )
    }

    fn call(&self, method: MethodName, payload: Blob) -> RResult<Blob, RString> {
        match method.as_str() {
            "import_html_v1" => import_txt(&payload.into_vec(), "html", "kalitech.text.meta.v1").map(|v| v),
            _ => err(format!("textimporter(html): unknown method '{method}'")),
        }
    }
}

#[derive(StableAbi)]
#[repr(C)]
struct TextTxtService;

impl ServiceV1 for TextTxtService {
    fn id(&self) -> RString {
        RString::from("kalitech.import.txt.v1")
    }

    fn describe(&self) -> RString {
        RString::from(
            r#"{
  "id":"kalitech.import.txt.v1",
  "kind":"asset_importer",
  "asset_importer":{
    "extensions":["txt","md"],
    "output_type_id":"kalitech.asset.text",
    "format":"txt",
    "method":"import_txt_v1",
    "wire":"u32_meta_len_le + meta_utf8 + payload_utf8"
  },
  "meta_schema":"kalitech.text.meta.v1"
}"#,
        )
    }

    fn call(&self, method: MethodName, payload: Blob) -> RResult<Blob, RString> {
        match method.as_str() {
            "import_txt_v1" => import_txt(&payload.into_vec(), "txt", "kalitech.text.meta.v1").map(|v| v),
            _ => err(format!("textimporter(txt): unknown method '{method}'")),
        }
    }
}

#[derive(StableAbi)]
#[repr(C)]
struct TextUiService;

impl ServiceV1 for TextUiService {
    fn id(&self) -> RString {
        RString::from("kalitech.import.ui.v1")
    }

    fn describe(&self) -> RString {
        RString::from(
            r#"{
  "id":"kalitech.import.ui.v1",
  "kind":"asset_importer",
  "asset_importer":{
    "extensions":["ui"],
    "output_type_id":"kalitech.asset.text",
    "format":"ui",
    "method":"import_ui_v1",
    "wire":"u32_meta_len_le + meta_utf8 + payload_utf8"
  },
  "meta_schema":"kalitech.text.meta.v1"
}"#,
        )
    }

    fn call(&self, method: MethodName, payload: Blob) -> RResult<Blob, RString> {
        match method.as_str() {
            "import_ui_v1" => import_txt(&payload.into_vec(), "ui", "kalitech.text.meta.v1").map(|v| v),
            _ => err(format!("textimporter(ui): unknown method '{method}'")),
        }
    }
}

/* =============================================================================================
   Plugin module
   ============================================================================================= */

#[derive(Default)]
pub struct TextImporterPlugin;

impl PluginModule for TextImporterPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: RString::from("import.text"),
            name: RString::from("Text Importer"),
            version: RString::from(env!("CARGO_PKG_VERSION")),
        }
    }

    fn init(&mut self, host: HostApiV1) -> RResult<(), RString> {
        let svcs: [ServiceV1Dyn<'static>; 5] = [
            ServiceV1_TO::from_value(TextJsonService, TD_Opaque),
            ServiceV1_TO::from_value(TextXmlService, TD_Opaque),
            ServiceV1_TO::from_value(TextHtmlService, TD_Opaque),
            ServiceV1_TO::from_value(TextTxtService, TD_Opaque),
            ServiceV1_TO::from_value(TextUiService, TD_Opaque),
        ];

        for svc in svcs {
            let r = (host.register_service_v1)(svc);
            if let Err(e) = r.clone().into_result() {
                (host.log_warn)(RString::from(format!(
                    "textimporter: register_service_v1 failed: {e}"
                )));
                return RResult::RErr(RString::from(format!(
                    "textimporter: register_service_v1 failed: {e}"
                )));
            }
        }

        RResult::ROk(())
    }

    fn start(&mut self) -> RResult<(), RString> {
        RResult::ROk(())
    }

    fn fixed_update(&mut self, _dt: f32) -> RResult<(), RString> {
        RResult::ROk(())
    }

    fn update(&mut self, _dt: f32) -> RResult<(), RString> {
        RResult::ROk(())
    }

    fn render(&mut self, _dt: f32) -> RResult<(), RString> {
        RResult::ROk(())
    }

    fn shutdown(&mut self) {}
}