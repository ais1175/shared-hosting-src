use crate::models::RedeemOutcome;
use reqwest::header::{
    ACCEPT, ACCEPT_LANGUAGE, CONNECTION, CONTENT_TYPE, ORIGIN, REFERER, USER_AGENT,
};
use std::time::Duration;
use url::Url;

pub fn extract_voucher_hash(voucher_url: &str) -> Result<String, &'static str> {
    let parsed = Url::parse(voucher_url).map_err(|_| "INVALID_VOUCHER_URL")?;

    if parsed.scheme() != "https" {
        return Err("INVALID_VOUCHER_URL");
    }

    let host = parsed.host_str().ok_or("INVALID_VOUCHER_URL")?;
    if host != "gift.truemoney.com" {
        return Err("INVALID_VOUCHER_URL");
    }

    if parsed.path() != "/campaign/" {
        return Err("INVALID_VOUCHER_URL");
    }

    let mut hashes = parsed.query_pairs().filter_map(|(key, value)| {
        if key == "v" {
            Some(value.into_owned())
        } else {
            None
        }
    });

    let hash = hashes.next().ok_or("INVALID_VOUCHER_URL")?;
    if hashes.next().is_some() {
        return Err("INVALID_VOUCHER_URL");
    }

    if hash.is_empty() || hash.len() > 128 || !hash.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Err("INVALID_VOUCHER_HASH");
    }

    Ok(hash)
}

pub async fn redeem_voucher_live(
    voucher_hash: &str,
    receiver_phone: &str,
    timeout_ms: u64,
) -> RedeemOutcome {
    let endpoint = format!("https://gift.truemoney.com/campaign/vouchers/{voucher_hash}/redeem");

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return RedeemOutcome {
                success: false,
                amount_thb: 0.0,
                error_code: Some("HTTP_CLIENT_ERROR".to_owned()),
                message: format!("Failed to build HTTP client: {err}"),
            }
        }
    };

    let payload = serde_json::json!({
        "mobile": receiver_phone,
        "voucher_hash": voucher_hash,
    });

    let response = match client
        .post(endpoint)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .header(USER_AGENT, "Mozilla/5.0")
        .header(ORIGIN, "https://gift.truemoney.com")
        .header(REFERER, "https://gift.truemoney.com/")
        .header(ACCEPT_LANGUAGE, "th,en;q=0.9")
        .header(CONNECTION, "keep-alive")
        .json(&payload)
        .send()
        .await
    {
        Ok(result) => result,
        Err(err) => {
            return RedeemOutcome {
                success: false,
                amount_thb: 0.0,
                error_code: Some("NETWORK_ERROR".to_owned()),
                message: format!("Connection error: {err}"),
            }
        }
    };

    let response_json = match response.json::<serde_json::Value>().await {
        Ok(value) => value,
        Err(err) => {
            return RedeemOutcome {
                success: false,
                amount_thb: 0.0,
                error_code: Some("INVALID_API_RESPONSE".to_owned()),
                message: format!("Invalid API response: {err}"),
            }
        }
    };

    parse_truemoney_response(&response_json)
}

fn parse_truemoney_response(payload: &serde_json::Value) -> RedeemOutcome {
    let status_code = match payload
        .get("status")
        .and_then(|value| value.get("code"))
        .and_then(|value| value.as_str())
    {
        Some(code) => code,
        None => {
            return RedeemOutcome {
                success: false,
                amount_thb: 0.0,
                error_code: Some("INVALID_API_RESPONSE".to_owned()),
                message: "Invalid API response".to_owned(),
            }
        }
    };

    if status_code == "SUCCESS" {
        let amount = payload
            .get("data")
            .and_then(|value| value.get("voucher"))
            .and_then(|value| value.get("redeemed_amount_baht"))
            .and_then(|value| {
                value
                    .as_str()
                    .and_then(|as_str| as_str.parse::<f64>().ok())
                    .or_else(|| value.as_f64())
            })
            .unwrap_or(0.0);

        return RedeemOutcome {
            success: true,
            amount_thb: round_money(amount),
            error_code: None,
            message: format!("รับซองอั่งเปาสำเร็จ {} บาท", round_money(amount)),
        };
    }

    RedeemOutcome {
        success: false,
        amount_thb: 0.0,
        error_code: Some(status_code.to_owned()),
        message: map_truemoney_error(status_code),
    }
}

fn map_truemoney_error(code: &str) -> String {
    match code {
        "MISSING_RECEIVER_PHONE_NUMBER" => "กรุณากรอกเบอร์รับซองอั่งเปา".to_owned(),
        "MISSING_GIFT_CODE_OR_URL" => "กรุณากรอกลิงก์ซองอั่งเปาหรือเบอร์โทรศัพท์ให้ถูกต้อง".to_owned(),
        "INVALID_RECEIVER_PHONE_NUMBER_FORMAT" => "รูปแบบเบอร์ไม่ถูกต้อง".to_owned(),
        "INVALID_GIFT_CODE_OR_URL_FORMAT" => "รูปแบบลิงก์ซองอั่งเปาไม่ถูกต้อง".to_owned(),
        "CANNOT_GET_OWN_VOUCHER" => "ไม่สามารถรับซองอั่งเปาของตัวเองได้".to_owned(),
        "VOUCHER_OUT_OF_STOCK" => "ซองอั่งเปานี้ถูกรับไปหมดแล้ว".to_owned(),
        "VOUCHER_EXPIRED" => "ซองอั่งเปาหมดอายุแล้ว".to_owned(),
        "UNEXPECTED_ERROR" => "เกิดข้อผิดพลาดในการรับซองอั่งเปา".to_owned(),
        _ => format!("เกิดข้อผิดพลาด: {code}"),
    }
}

pub fn round_money(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub fn mask_voucher_hash(voucher_hash: &str) -> String {
    let visible_len = voucher_hash.len().min(6);
    let suffix = &voucher_hash[voucher_hash.len() - visible_len..];
    format!("***{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_response_success_amount() {
        let payload = serde_json::json!({
            "status": { "code": "SUCCESS" },
            "data": { "voucher": { "redeemed_amount_baht": "50" } }
        });

        let result = parse_truemoney_response(&payload);
        assert!(result.success);
        assert_eq!(result.amount_thb, 50.0);
        assert_eq!(result.error_code, None);
        assert!(result.message.contains("รับซองอั่งเปาสำเร็จ"));
    }

    #[test]
    fn parse_response_known_error() {
        let payload = serde_json::json!({
            "status": { "code": "VOUCHER_OUT_OF_STOCK" }
        });

        let result = parse_truemoney_response(&payload);
        assert!(!result.success);
        assert_eq!(result.amount_thb, 0.0);
        assert_eq!(result.error_code.as_deref(), Some("VOUCHER_OUT_OF_STOCK"));
        assert_eq!(result.message, map_truemoney_error("VOUCHER_OUT_OF_STOCK"));
    }

    #[test]
    fn parse_response_invalid_payload() {
        let payload = serde_json::json!({ "foo": "bar" });
        let result = parse_truemoney_response(&payload);
        assert!(!result.success);
        assert_eq!(result.amount_thb, 0.0);
        assert_eq!(result.error_code.as_deref(), Some("INVALID_API_RESPONSE"));
    }

    #[test]
    fn parse_response_unknown_error_contains_error_code() {
        let payload = serde_json::json!({
            "status": { "code": "SOME_NEW_ERROR_CODE" }
        });
        let result = parse_truemoney_response(&payload);

        assert!(!result.success);
        assert_eq!(result.amount_thb, 0.0);
        assert_eq!(result.error_code.as_deref(), Some("SOME_NEW_ERROR_CODE"));
        assert!(result.message.contains("SOME_NEW_ERROR_CODE"));
    }
}
