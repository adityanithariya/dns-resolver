use dns_core::message::{Message, QType, formatter};
use serde_wasm_bindgen;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[wasm_bindgen]
pub fn encode_query(id: u16, name: &str, qtype: u16) -> Result<Vec<u8>, JsValue> {
    let qtype = QType::from_u16(qtype);

    let message = Message::new_query(id, name, qtype);

    Ok(message.encode())
}

#[wasm_bindgen]
pub fn decode_message(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let message = Message::decode(bytes).map_err(|e| JsValue::from_str(&format!("{e:?}")))?;

    serde_wasm_bindgen::to_value(&message).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn format_message(bytes: &[u8]) -> Result<String, JsValue> {
    let message = Message::decode(bytes).map_err(|e| JsValue::from_str(&format!("{e:?}")))?;

    Ok(formatter::format_message(&message))
}
