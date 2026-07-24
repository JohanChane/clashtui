use super::*;

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32);
        out.push(B64[((n >> 18) & 0x3F) as usize] as char);
        out.push(B64[((n >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            B64[((n >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[(n & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn strip_userinfo(url: &str) -> (String, Option<String>) {
    let Some(scheme_end) = url.find("://") else {
        return (url.to_string(), None);
    };
    let rest = &url[(scheme_end + 3)..];
    let at_pos = rest.find('@');
    let slash_pos = rest.find('/');
    let is_in_authority = match (at_pos, slash_pos) {
        (Some(a), Some(s)) => a < s,
        (Some(_), None) => true,
        _ => false,
    };
    if !is_in_authority {
        return (url.to_string(), None);
    }
    let userinfo = &rest[..at_pos.unwrap()];
    let auth_value = if userinfo.contains(':') {
        userinfo.to_string()
    } else {
        format!("{userinfo}:")
    };
    let auth_header = format!("Basic {}", base64_encode(auth_value.as_bytes()));
    let prefix = &url[..(scheme_end + 3)];
    let suffix = &rest[(at_pos.unwrap() + 1)..];
    (format!("{prefix}{suffix}"), Some(auth_header))
}

pub fn profile(url: &str, with_proxy: bool) -> Result<minreq::ResponseLazy> {
    let (clean_url, auth_header) = strip_userinfo(url);
    let mut req = minreq::get(&clean_url);
    if with_proxy {
        req = req.with_proxy(minreq::Proxy::new(&CONFIG.proxy_addr)?)
    }
    req = req.with_timeout(timeout!()).with_header(
        headers::USER_AGENT,
        CONFIG.global_ua.as_deref().unwrap_or_else(|| {
            if CONFIG.core_type() == crate::config::CoreType::Singbox {
                "sing-box"
            } else {
                "clash.meta"
            }
        }),
    );
    if let Some(auth) = auth_header {
        req = req.with_header(headers::AUTHORIZATION, auth);
    }
    req.send_lazy()
}

pub fn fetch_subscription_userinfo(url: &str, with_proxy: bool) -> Result<Option<String>> {
    let (clean_url, auth_header) = strip_userinfo(url);
    let mut req = minreq::get(&clean_url)
        .with_timeout(timeout!())
        .with_header(
            headers::USER_AGENT,
            CONFIG.global_ua.as_deref().unwrap_or_else(|| {
                if CONFIG.core_type() == crate::config::CoreType::Singbox {
                    "sing-box"
                } else {
                    "clash.meta"
                }
            }),
        );
    if with_proxy {
        req = req.with_proxy(minreq::Proxy::new(&CONFIG.proxy_addr)?);
    }
    if let Some(auth) = auth_header {
        req = req.with_header(headers::AUTHORIZATION, auth);
    }
    let resp = req.send()?;
    let info: Option<String> = resp.headers.get("subscription-userinfo").cloned();
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_token_from_github_url() {
        let url = "https://ghp_token@raw.githubusercontent.com/user/repo/main/config.yaml";
        let (clean, auth) = strip_userinfo(url);
        assert_eq!(
            clean,
            "https://raw.githubusercontent.com/user/repo/main/config.yaml"
        );
        assert_eq!(auth.unwrap(), "Basic Z2hwX3Rva2VuOg==");
    }

    #[test]
    fn strip_user_pass_from_url() {
        let url = "https://user:pass@example.com/path";
        let (clean, auth) = strip_userinfo(url);
        assert_eq!(clean, "https://example.com/path");
        assert_eq!(auth.unwrap(), "Basic dXNlcjpwYXNz");
    }

    #[test]
    fn no_userinfo_no_change() {
        let url = "https://example.com/path";
        let (clean, auth) = strip_userinfo(url);
        assert_eq!(clean, url);
        assert!(auth.is_none());
    }

    #[test]
    fn at_in_path_not_userinfo() {
        let url = "https://example.com/path?q=@test";
        let (clean, auth) = strip_userinfo(url);
        assert_eq!(clean, url);
        assert!(auth.is_none());
    }
}
