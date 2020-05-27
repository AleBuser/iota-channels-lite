use serde;
use serde::Serialize;

///
/// Object returned by write_signed
///
#[derive(Serialize)]
pub struct ResponseSigned {
    pub signed_message_tag: String,
    pub change_key_tag: Option<String>,
}
