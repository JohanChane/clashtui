use super::*;

macro_rules! timeout {
    () => {
        CONFIG.cfg_file.timeout.unwrap_or(DEFAULT_TIMEOUT)
    };
}

pub fn request_return_204(method: Method, sub_url: &str, payload: Option<String>) -> Result<()> {
    let rep = request(method, sub_url, payload)?;
    match rep.status_code {
        204 => Ok(()),
        _ => Err(minreq::Error::IoError(std::io::Error::other(format!(
            "Expect 204, got {rep:?}"
        )))),
    }
}

pub type WSerr = tokio::sync::oneshot::Receiver<minreq::Error>;

pub fn listen_ws<T, F>(
    sub_url: impl AsRef<str>,
    mut f: F,
    pause: &'static tokio::sync::Semaphore,
) -> Result<WSerr>
where
    for<'a> T: serde::Deserialize<'a>,
    F: FnMut(T) -> () + Send + 'static,
{
    let controller = CONFIG.controller_for_core();
    let endpoint = format!("{controller}{}", sub_url.as_ref());
    let mut rdr = minreq::get(endpoint).send_lazy()?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        while let Some(maybe_byte) = rdr.next() {
            match || -> Result<T> {
                let (byte, size) = maybe_byte.unwrap();
                let bytes: Result<Vec<u8>> = std::iter::once(Ok(byte))
                    .chain(rdr.by_ref().take(size - 1).map(|r| r.map(|(b, _)| b)))
                    .collect();
                let bytes = bytes.unwrap();
                serde_json::from_slice(&bytes).map_err(|e| minreq::Error::SerdeJsonError(e))
            }() {
                Ok(_) if pause.available_permits() == 0 => {}
                Ok(msg) => f(msg),
                Err(e) => {
                    tx.send(e).unwrap();
                    return;
                }
            }
        }
    });

    Ok(rx)
}

pub fn request(
    method: minreq::Method,
    sub_url: &str,
    payload: Option<String>,
) -> Result<minreq::Response> {
    let controller = CONFIG.controller_for_core();
    let endpoint = format!("{controller}{sub_url}");

    let mut req = minreq::Request::new(method, endpoint);
    if let Some(kv) = payload {
        req = req
            .with_header(headers::CONTENT_TYPE, "application/json")
            .with_body(kv);
    }
    if let Some(s) = CONFIG.secret_for_core() {
        req = req.with_header(headers::AUTHORIZATION, format!("Bearer {s}"));
    }
    req.with_timeout(timeout!()).send()
}
