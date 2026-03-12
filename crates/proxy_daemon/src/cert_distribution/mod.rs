mod handlers;
mod template;
mod types;

pub(crate) use handlers::{handle_cert_request, is_cert_download_request};

#[cfg(test)]
mod tests;
