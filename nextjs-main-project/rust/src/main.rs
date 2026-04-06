mod auth;
mod models;
mod storage;
mod truemoney;

use auth::{
    build_clear_refresh_cookie, build_refresh_cookie, extract_bearer_token, hash_refresh_token,
    issue_access_token, new_refresh_token, read_cookie, validate_access_token, AuthConfig,
    AuthError, UserRole,
};
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{header::SET_COOKIE, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Datelike, Duration as ChronoDuration, NaiveDate, Timelike, Utc};
use models::{
    AdminLoginRequest, AdminServiceView, AdminServicesResponse, AdminSummaryResponse,
    AdminTransactionView, AdminTransactionsResponse, ApiResponse, AuthSessionPayload,
    AdminUserWalletView, AdminUserWalletsResponse,
    BankingSlipRedeemRequest, CreateHostingOrderRequest, CreateHostingOrderResponse, ErrorResponse,
    HostingServiceItem, HostingServicesResponse, LoginRequest, LoginResponse,
    MarkReadNotificationsResponse, NotificationItem, NotificationsResponse, RedeemRequest,
    RedeemResponse, RefreshResponse, RefreshSession, RegisterSessionRequest,
    RenewHostingServiceRequest, RenewHostingServiceResponse, Session, SessionView,
    TopupTransaction, TopupTransactionView, TransactionsQuery, TransactionsResponse, WalletAccount,
    WalletResponse,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use storage::{load_json_from_file, save_json_to_file_atomic};
use tokio::sync::{Mutex, RwLock};
use tower_http::cors::{Any, CorsLayer};
use truemoney::{extract_voucher_hash, mask_voucher_hash, redeem_voucher_live, round_money};
use uuid::Uuid;

const LOGIN_PEPPER: &str = "reverz-wasm-pepper-v1";
const DEFAULT_BANKING_RECEIVER_ID: &str = "xxx-x-x8407-x";
const DEFAULT_BANKING_RECEIVER_NAME: &str = "MASTER MATHAKAN TONGEAM";
const DEFAULT_BANKING_SLIP_API_URL: &str = "https://slip-c.oiioioiiioooioio.download/api/slip";
const BANGKOK_TIME_API_URL: &str = "https://timeapi.io/api/Time/current/zone?timeZone=Asia/Bangkok";
const BANKING_SLIP_MAX_IMG_CHARS: usize = 14 * 1024 * 1024;
const SERVICE_STATUS_ACTIVE: &str = "active";
const SERVICE_STATUS_GRACE_SUSPENDED: &str = "grace_suspended";
const SERVICE_STATUS_SUSPENDED_EXPIRED: &str = "suspended_expired";

#[derive(Debug, Serialize)]
struct SlipApiRequest<'a> {
    img: &'a str,
    tos: bool,
    privacy: bool,
    eula: bool,
}

#[derive(Debug, Deserialize)]
struct SlipApiResponse {
    data: SlipApiResponseData,
}

#[derive(Debug, Deserialize)]
struct SlipApiResponseData {
    #[serde(rename = "ref")]
    ref_id: String,
    date: String,
    amount: serde_json::Value,
    receiver_name: String,
    receiver_id: String,
}

#[derive(Debug, Deserialize)]
struct BangkokTimeApiResponse {
    date: String,
}

#[derive(Debug, Deserialize)]
struct NotificationsQuery {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct AdminListQuery {
    limit: Option<usize>,
}

#[derive(Clone)]
struct AppState {
    sessions_by_user: Arc<RwLock<HashMap<String, Vec<Session>>>>,
    wallets_by_user: Arc<RwLock<HashMap<String, WalletAccount>>>,
    transactions_by_user: Arc<RwLock<HashMap<String, Vec<TopupTransaction>>>>,
    hosting_services_by_user: Arc<RwLock<HashMap<String, Vec<HostingServiceItem>>>>,
    notifications_by_user: Arc<RwLock<HashMap<String, Vec<NotificationItem>>>>,
    redeemed_vouchers: Arc<RwLock<HashSet<String>>>,
    used_slip_refs: Arc<RwLock<HashSet<String>>>,
    refresh_sessions: Arc<RwLock<HashMap<String, RefreshSession>>>,
    rate_limit_hits: Arc<RwLock<HashMap<String, Vec<u64>>>>,
    sessions_storage_file: Arc<PathBuf>,
    wallets_storage_file: Arc<PathBuf>,
    transactions_storage_file: Arc<PathBuf>,
    hosting_services_storage_file: Arc<PathBuf>,
    notifications_storage_file: Arc<PathBuf>,
    redeemed_storage_file: Arc<PathBuf>,
    used_slip_refs_storage_file: Arc<PathBuf>,
    refresh_sessions_storage_file: Arc<PathBuf>,
    persistence_lock: Arc<Mutex<()>>,
    topup_commit_lock: Arc<Mutex<()>>,
    receiver_phone: Arc<String>,
    banking_receiver_id: Arc<String>,
    banking_receiver_name: Arc<String>,
    banking_slip_api_url: Arc<String>,
    truemoney_timeout_ms: u64,
    banking_slip_timeout_ms: u64,
    rate_limit_per_minute: usize,
    auth_config: Arc<AuthConfig>,
    admin_username: Arc<String>,
    admin_password: Arc<String>,
    admin_login_email: Arc<String>,
    admin_login_password: Arc<String>,
    da_url: Arc<String>,
    da_username: Arc<String>,
    da_password: Arc<String>,
    da_server_ip: Arc<String>,
}

#[tokio::main]
async fn main() {
    let state = match build_state().await {
        Ok(state) => state,
        Err(err) => {
            eprintln!("failed to build app state: {err}");
            std::process::exit(1);
        }
    };

    let addr: SocketAddr = "0.0.0.0:8081".parse().expect("invalid bind address");
    println!("session/topup api listening on http://{addr}");

    let lifecycle_state = state.clone();
    tokio::spawn(async move {
        lifecycle_worker(lifecycle_state).await;
    });

    let app = build_app(state);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind listener");

    axum::serve(listener, app).await.expect("server failed");
}

async fn build_state() -> Result<AppState, String> {
    let sessions_storage_file = PathBuf::from("rust/storage/sessions.json");
    let wallets_storage_file = PathBuf::from("rust/storage/wallets.json");
    let transactions_storage_file = PathBuf::from("rust/storage/topup_transactions.json");
    let hosting_services_storage_file = PathBuf::from("rust/storage/hosting_services.json");
    let notifications_storage_file = PathBuf::from("rust/storage/notifications.json");
    let redeemed_storage_file = PathBuf::from("rust/storage/redeemed_vouchers.json");
    let used_slip_refs_storage_file = PathBuf::from("rust/storage/used_slip_refs.json");
    let refresh_sessions_storage_file = PathBuf::from("rust/storage/refresh_sessions.json");

    let sessions = load_json_from_file::<HashMap<String, Vec<Session>>>(&sessions_storage_file)
        .unwrap_or_default();
    let wallets = load_json_from_file::<HashMap<String, WalletAccount>>(&wallets_storage_file)
        .unwrap_or_default();
    let transactions =
        load_json_from_file::<HashMap<String, Vec<TopupTransaction>>>(&transactions_storage_file)
            .unwrap_or_default();
    let mut hosting_services = load_json_from_file::<HashMap<String, Vec<HostingServiceItem>>>(
        &hosting_services_storage_file,
    )
    .unwrap_or_default();
    let notifications =
        load_json_from_file::<HashMap<String, Vec<NotificationItem>>>(&notifications_storage_file)
            .unwrap_or_default();
    let mut redeemed =
        load_json_from_file::<HashSet<String>>(&redeemed_storage_file).unwrap_or_default();
    let mut used_slip_refs =
        load_json_from_file::<HashSet<String>>(&used_slip_refs_storage_file).unwrap_or_default();
    let refresh_sessions =
        load_json_from_file::<HashMap<String, RefreshSession>>(&refresh_sessions_storage_file)
            .unwrap_or_default();

    for list in transactions.values() {
        for item in list {
            if item.status == "success" {
                redeemed.insert(item.voucher_hash.to_owned());
                if let Some(reference_id) = item.voucher_hash.strip_prefix("slip:") {
                    used_slip_refs.insert(normalize_slip_ref(reference_id));
                }
            }
        }
    }

    let receiver_phone = require_env("TRUEMONEY_RECEIVER_PHONE")?;
    validate_receiver_phone(&receiver_phone)?;

    let jwt_secret = require_env("JWT_SECRET")?;
    let refresh_secret = require_env("REFRESH_TOKEN_SECRET")?;

    let access_ttl_seconds = parse_env_u64("ACCESS_TOKEN_TTL_SECONDS", 86_400);
    let refresh_ttl_seconds = parse_env_u64("REFRESH_TOKEN_TTL_SECONDS", 2_592_000);
    let truemoney_timeout_ms = parse_env_u64("TRUEMONEY_TIMEOUT_MS", 10_000);
    let banking_slip_timeout_ms = parse_env_u64("BANKING_SLIP_TIMEOUT_MS", 10_000);
    let rate_limit_per_minute = parse_env_usize("TOPUP_RATE_LIMIT_PER_MINUTE", 5);
    let banking_receiver_id = parse_env_string("BANKING_RECEIVER_ID", DEFAULT_BANKING_RECEIVER_ID);
    let banking_receiver_name =
        parse_env_string("BANKING_RECEIVER_NAME", DEFAULT_BANKING_RECEIVER_NAME);
    let banking_slip_api_url =
        parse_env_string("BANKING_SLIP_API_URL", DEFAULT_BANKING_SLIP_API_URL);

    let secure_cookies = std::env::var("COOKIE_SECURE")
        .ok()
        .map(|raw| raw == "1" || raw.eq_ignore_ascii_case("true"))
        .unwrap_or(true);

    // Keep a fixed demo account for local testing.
    let admin_username = "root".to_owned();
    let admin_password = "root".to_owned();
    let admin_login_email = require_env("ADMIN_LOGIN_EMAIL")?;
    let admin_login_password = require_env("ADMIN_LOGIN_PASSWORD")?;

    let da_url =
        normalize_da_panel_url(&parse_env_string("DA_URL", "https://dcadmin.reverz.in.th"));
    let da_username = parse_env_string("DA_USERNAME", "");
    let da_password = parse_env_string("DA_PASSWORD", "");
    let da_server_ip = parse_env_string("DA_SERVER_IP", "158.173.159.171");
    let mut hosting_services_changed = false;

    for list in hosting_services.values_mut() {
        if backfill_hosting_service_list(list, &da_url) {
            hosting_services_changed = true;
        }
    }

    for (username, txs) in &transactions {
        let inferred = collect_hosting_services(txs, &da_url);
        if inferred.is_empty() {
            continue;
        }
        let list = hosting_services.entry(username.to_owned()).or_default();
        for mut inferred_item in inferred {
            if list
                .iter()
                .any(|existing| existing.domain.eq_ignore_ascii_case(&inferred_item.domain))
            {
                continue;
            }
            backfill_hosting_service_defaults(&mut inferred_item, &da_url);
            list.push(inferred_item);
            hosting_services_changed = true;
        }
        if backfill_hosting_service_list(list, &da_url) {
            hosting_services_changed = true;
        }
    }

    if hosting_services_changed {
        save_json_to_file_atomic(&hosting_services_storage_file, &hosting_services)?;
    }

    Ok(AppState {
        sessions_by_user: Arc::new(RwLock::new(sessions)),
        wallets_by_user: Arc::new(RwLock::new(wallets)),
        transactions_by_user: Arc::new(RwLock::new(transactions)),
        hosting_services_by_user: Arc::new(RwLock::new(hosting_services)),
        notifications_by_user: Arc::new(RwLock::new(notifications)),
        redeemed_vouchers: Arc::new(RwLock::new(redeemed)),
        used_slip_refs: Arc::new(RwLock::new(used_slip_refs)),
        refresh_sessions: Arc::new(RwLock::new(refresh_sessions)),
        rate_limit_hits: Arc::new(RwLock::new(HashMap::new())),
        sessions_storage_file: Arc::new(sessions_storage_file),
        wallets_storage_file: Arc::new(wallets_storage_file),
        transactions_storage_file: Arc::new(transactions_storage_file),
        hosting_services_storage_file: Arc::new(hosting_services_storage_file),
        notifications_storage_file: Arc::new(notifications_storage_file),
        redeemed_storage_file: Arc::new(redeemed_storage_file),
        used_slip_refs_storage_file: Arc::new(used_slip_refs_storage_file),
        refresh_sessions_storage_file: Arc::new(refresh_sessions_storage_file),
        persistence_lock: Arc::new(Mutex::new(())),
        topup_commit_lock: Arc::new(Mutex::new(())),
        receiver_phone: Arc::new(receiver_phone),
        banking_receiver_id: Arc::new(banking_receiver_id),
        banking_receiver_name: Arc::new(banking_receiver_name),
        banking_slip_api_url: Arc::new(banking_slip_api_url),
        truemoney_timeout_ms,
        banking_slip_timeout_ms,
        rate_limit_per_minute,
        auth_config: Arc::new(AuthConfig {
            jwt_secret,
            refresh_secret,
            access_ttl_seconds,
            refresh_ttl_seconds,
            secure_cookies,
        }),
        admin_username: Arc::new(admin_username),
        admin_password: Arc::new(admin_password),
        admin_login_email: Arc::new(admin_login_email),
        admin_login_password: Arc::new(admin_login_password),
        da_url: Arc::new(da_url),
        da_username: Arc::new(da_username),
        da_password: Arc::new(da_password),
        da_server_ip: Arc::new(da_server_ip),
    })
}

fn build_app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health))
        .route("/login", post(login))
        .route("/admin/login", post(admin_login))
        .route("/auth/refresh", post(refresh_access_token))
        .route("/logout", post(logout))
        .route("/sessions", get(get_sessions))
        .route("/sessions/register", post(register_session))
        .route("/sessions/{id}/revoke", post(revoke_session))
        .route("/topup/wallet", get(get_wallet))
        .route("/topup/transactions", get(get_transactions))
        .route("/topup/truemoney/redeem", post(redeem_truemoney))
        .route("/topup/banking/slip/redeem", post(redeem_banking_slip))
        .route("/hosting/order", post(create_hosting_order))
        .route("/hosting/services", get(list_hosting_services))
        .route("/hosting/services/renew", post(renew_hosting_service))
        .route("/notifications", get(get_notifications))
        .route("/notifications/mark-read", post(mark_notifications_read))
        .route("/admin/summary", get(admin_summary))
        .route("/admin/recent-services", get(admin_recent_services))
        .route("/admin/recent-transactions", get(admin_recent_transactions))
        .route("/admin/user-wallets", get(admin_user_wallets))
        .layer(DefaultBodyLimit::max(
            BANKING_SLIP_MAX_IMG_CHARS + 1_048_576,
        ))
        .with_state(state)
        .layer(cors)
}

async fn health() -> impl IntoResponse {
    Json(ApiResponse {
        ok: true,
        message: "ok".to_owned(),
    })
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    if payload.username != *state.admin_username || payload.password != *state.admin_password {
        return error_response(
            StatusCode::UNAUTHORIZED,
            "Invalid credentials",
            Some("INVALID_CREDENTIALS"),
        );
    }

    if payload.nonce.trim().is_empty() || payload.proof.trim().is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Missing nonce or proof",
            Some("INVALID_LOGIN_PROOF"),
        );
    }

    let expected = compute_login_proof(&payload.username, &payload.password, &payload.nonce);
    if expected != payload.proof {
        return error_response(
            StatusCode::UNAUTHORIZED,
            "Invalid login proof",
            Some("INVALID_LOGIN_PROOF"),
        );
    }

    let now_unix = unix_now_secs();
    let issued_at = now_iso();
    let access_token = match issue_access_token(
        &payload.username,
        UserRole::Root,
        state.auth_config.as_ref(),
        now_unix,
    ) {
        Ok(token) => token,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to issue access token: {err}"),
                Some("TOKEN_ISSUE_ERROR"),
            )
        }
    };

    let refresh_token = new_refresh_token();
    let refresh_hash = hash_refresh_token(&refresh_token, &state.auth_config.refresh_secret);
    let family_id = Uuid::new_v4().to_string();

    {
        let mut refresh_sessions = state.refresh_sessions.write().await;
        refresh_sessions.insert(
            refresh_hash,
            RefreshSession {
                username: payload.username.to_owned(),
                role: UserRole::Root.as_str().to_owned(),
                family_id,
                issued_at_unix: now_unix,
                expires_at_unix: now_unix.saturating_add(state.auth_config.refresh_ttl_seconds),
                revoked_at_unix: None,
            },
        );
    }

    if let Err(err) = persist_refresh_sessions(&state).await {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to persist refresh session: {err}"),
            Some("PERSISTENCE_ERROR"),
        );
    }

    let cookie = build_refresh_cookie(
        &refresh_token,
        state.auth_config.refresh_ttl_seconds,
        state.auth_config.secure_cookies,
    );

    json_with_cookie(
        StatusCode::OK,
        LoginResponse {
            ok: true,
            session: AuthSessionPayload {
                username: payload.username,
                role: "root".to_owned(),
                logged_in_at: issued_at,
                access_token,
            },
        },
        &cookie,
    )
}

async fn admin_login(
    State(state): State<AppState>,
    Json(payload): Json<AdminLoginRequest>,
) -> impl IntoResponse {
    if payload.email.trim() != *state.admin_login_email
        || payload.password != *state.admin_login_password
    {
        return error_response(
            StatusCode::UNAUTHORIZED,
            "Invalid credentials",
            Some("INVALID_CREDENTIALS"),
        );
    }

    if payload.nonce.trim().is_empty() || payload.proof.trim().is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Missing nonce or proof",
            Some("INVALID_LOGIN_PROOF"),
        );
    }

    let expected = compute_login_proof(&payload.email, &payload.password, &payload.nonce);
    if expected != payload.proof {
        return error_response(
            StatusCode::UNAUTHORIZED,
            "Invalid login proof",
            Some("INVALID_LOGIN_PROOF"),
        );
    }

    let now_unix = unix_now_secs();
    let issued_at = now_iso();
    let access_token = match issue_access_token(
        &payload.email,
        UserRole::Admin,
        state.auth_config.as_ref(),
        now_unix,
    ) {
        Ok(token) => token,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to issue access token: {err}"),
                Some("TOKEN_ISSUE_ERROR"),
            )
        }
    };

    let refresh_token = new_refresh_token();
    let refresh_hash = hash_refresh_token(&refresh_token, &state.auth_config.refresh_secret);
    let family_id = Uuid::new_v4().to_string();

    {
        let mut refresh_sessions = state.refresh_sessions.write().await;
        refresh_sessions.insert(
            refresh_hash,
            RefreshSession {
                username: payload.email.to_owned(),
                role: UserRole::Admin.as_str().to_owned(),
                family_id,
                issued_at_unix: now_unix,
                expires_at_unix: now_unix.saturating_add(state.auth_config.refresh_ttl_seconds),
                revoked_at_unix: None,
            },
        );
    }

    if let Err(err) = persist_refresh_sessions(&state).await {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to persist refresh session: {err}"),
            Some("PERSISTENCE_ERROR"),
        );
    }

    let cookie = build_refresh_cookie(
        &refresh_token,
        state.auth_config.refresh_ttl_seconds,
        state.auth_config.secure_cookies,
    );

    json_with_cookie(
        StatusCode::OK,
        LoginResponse {
            ok: true,
            session: AuthSessionPayload {
                username: payload.email,
                role: UserRole::Admin.as_str().to_owned(),
                logged_in_at: issued_at,
                access_token,
            },
        },
        &cookie,
    )
}

async fn refresh_access_token(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let refresh_token = match read_cookie(&headers, "refresh_token") {
        Some(token) => token,
        None => {
            return error_response(
                StatusCode::UNAUTHORIZED,
                "Missing refresh token",
                Some("UNAUTHENTICATED"),
            )
        }
    };

    let refresh_hash = hash_refresh_token(&refresh_token, &state.auth_config.refresh_secret);
    let now_unix = unix_now_secs();

    let (username, role, rotated_token) = {
        let mut sessions = state.refresh_sessions.write().await;
        let session = match sessions.get_mut(&refresh_hash) {
            Some(found) => found,
            None => {
                return error_response(
                    StatusCode::UNAUTHORIZED,
                    "Invalid refresh token",
                    Some("INVALID_REFRESH_TOKEN"),
                )
            }
        };

        if session.revoked_at_unix.is_some() || now_unix >= session.expires_at_unix {
            let family_id = session.family_id.to_owned();
            revoke_refresh_family(&mut sessions, &family_id, now_unix);
            drop(sessions);
            let _ = persist_refresh_sessions(&state).await;
            return error_response(
                StatusCode::UNAUTHORIZED,
                "Invalid refresh token",
                Some("INVALID_REFRESH_TOKEN"),
            );
        }

        let username = session.username.to_owned();
        let role = session.role.to_owned();
        let family_id = session.family_id.to_owned();
        session.revoked_at_unix = Some(now_unix);

        let new_token = new_refresh_token();
        let new_hash = hash_refresh_token(&new_token, &state.auth_config.refresh_secret);

        sessions.insert(
            new_hash,
            RefreshSession {
                username: username.to_owned(),
                role: role.to_owned(),
                family_id,
                issued_at_unix: now_unix,
                expires_at_unix: now_unix.saturating_add(state.auth_config.refresh_ttl_seconds),
                revoked_at_unix: None,
            },
        );

        (username, role, new_token)
    };

    if let Err(err) = persist_refresh_sessions(&state).await {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to persist refresh session: {err}"),
            Some("PERSISTENCE_ERROR"),
        );
    }

    let role_enum = if role.eq_ignore_ascii_case("admin") {
        UserRole::Admin
    } else {
        UserRole::Root
    };
    let access_token =
        match issue_access_token(&username, role_enum, state.auth_config.as_ref(), now_unix) {
            Ok(token) => token,
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Failed to issue access token: {err}"),
                    Some("TOKEN_ISSUE_ERROR"),
                )
            }
        };

    let cookie = build_refresh_cookie(
        &rotated_token,
        state.auth_config.refresh_ttl_seconds,
        state.auth_config.secure_cookies,
    );

    json_with_cookie(
        StatusCode::OK,
        RefreshResponse {
            ok: true,
            access_token,
        },
        &cookie,
    )
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let username = match require_any_user(&state, &headers) {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let refresh_token = read_cookie(&headers, "refresh_token");
    let now_unix = unix_now_secs();

    {
        let mut sessions = state.refresh_sessions.write().await;
        if let Some(token) = refresh_token {
            let hash = hash_refresh_token(&token, &state.auth_config.refresh_secret);
            if let Some(session) = sessions.get(&hash) {
                let family_id = session.family_id.to_owned();
                revoke_refresh_family(&mut sessions, &family_id, now_unix);
            }
        } else {
            revoke_refresh_user(&mut sessions, &username, now_unix);
        }
    }

    if let Err(err) = persist_refresh_sessions(&state).await {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to persist refresh session: {err}"),
            Some("PERSISTENCE_ERROR"),
        );
    }

    let clear_cookie = build_clear_refresh_cookie(state.auth_config.secure_cookies);
    json_with_cookie(
        StatusCode::OK,
        ApiResponse {
            ok: true,
            message: "logged out".to_owned(),
        },
        &clear_cookie,
    )
}
async fn get_sessions(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let user = match require_access_user(&state, &headers) {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let current_session_id = headers
        .get("x-current-session-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();

    let data = state.sessions_by_user.read().await;
    let mut sessions = data
        .get(&user)
        .map(|items| {
            items
                .iter()
                .filter(|item| !item.revoked)
                .map(|item| SessionView {
                    id: item.id.to_owned(),
                    device: item.device.to_owned(),
                    ip: item.ip.to_owned(),
                    location: item.location.to_owned(),
                    last_active: item.last_active.to_owned(),
                    is_current: item.id == current_session_id,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    sessions.sort_by_key(|item| !item.is_current);
    (StatusCode::OK, Json(sessions)).into_response()
}

async fn register_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RegisterSessionRequest>,
) -> impl IntoResponse {
    let user = match require_access_user(&state, &headers) {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let mut sessions_map = state.sessions_by_user.write().await;
    let sessions = sessions_map.entry(user).or_default();

    if let Some(existing) = sessions
        .iter_mut()
        .find(|item| item.id == payload.session_id)
    {
        existing.device = payload.device;
        existing.ip = payload.ip;
        existing.location = payload.location;
        existing.last_active = payload.last_active;
        existing.revoked = false;
    } else {
        sessions.push(Session {
            id: payload.session_id,
            device: payload.device,
            ip: payload.ip,
            location: payload.location,
            last_active: payload.last_active,
            revoked: false,
        });
    }

    drop(sessions_map);

    if let Err(err) = persist_sessions(&state).await {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to persist session: {err}"),
            Some("PERSISTENCE_ERROR"),
        );
    }

    (
        StatusCode::OK,
        Json(ApiResponse {
            ok: true,
            message: "session registered".to_owned(),
        }),
    )
        .into_response()
}

async fn revoke_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match require_access_user(&state, &headers) {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let current_session_id = headers
        .get("x-current-session-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    let mut sessions_map = state.sessions_by_user.write().await;
    let sessions = sessions_map.entry(user).or_default();

    if let Some(target) = sessions
        .iter_mut()
        .find(|item| item.id == id && !item.revoked)
    {
        if target.id == current_session_id {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse {
                    ok: false,
                    message: "cannot revoke current session".to_owned(),
                }),
            )
                .into_response();
        }

        target.revoked = true;
        target.last_active = format!("revoked at {}", now_iso());
        drop(sessions_map);

        if let Err(err) = persist_sessions(&state).await {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to persist session: {err}"),
                Some("PERSISTENCE_ERROR"),
            );
        }

        return (
            StatusCode::OK,
            Json(ApiResponse {
                ok: true,
                message: "session revoked".to_owned(),
            }),
        )
            .into_response();
    }

    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse {
            ok: false,
            message: "session not found".to_owned(),
        }),
    )
        .into_response()
}

async fn get_wallet(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let user = match require_access_user(&state, &headers) {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let wallets = state.wallets_by_user.read().await;
    let balance = wallets
        .get(&user)
        .map(|wallet| wallet.balance_thb)
        .unwrap_or(0.0);

    (
        StatusCode::OK,
        Json(WalletResponse {
            ok: true,
            username: user,
            balance_thb: round_money(balance),
            receiver_phone: state.receiver_phone.as_ref().to_owned(),
            banking_receiver_id: state.banking_receiver_id.as_ref().to_owned(),
            banking_receiver_name: state.banking_receiver_name.as_ref().to_owned(),
        }),
    )
        .into_response()
}

async fn get_transactions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<TransactionsQuery>,
) -> impl IntoResponse {
    let user = match require_access_user(&state, &headers) {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let limit = query.limit.unwrap_or(20).min(100);
    let transactions = state.transactions_by_user.read().await;

    let mut items = transactions
        .get(&user)
        .map(|list| {
            list.iter()
                .rev()
                .take(limit)
                .map(|item| TopupTransactionView {
                    tx_id: item.tx_id.to_owned(),
                    voucher_hash: display_reference_id(&item.voucher_hash),
                    amount_thb: item.amount_thb,
                    status: item.status.to_owned(),
                    error_code: item.error_code.as_ref().map(|code| code.to_owned()),
                    message: item.message.to_owned(),
                    created_at: item.created_at.to_owned(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    items.sort_by(|left, right| right.created_at.cmp(&left.created_at));

    (
        StatusCode::OK,
        Json(TransactionsResponse { ok: true, items }),
    )
        .into_response()
}

async fn list_hosting_services(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match require_access_user(&state, &headers) {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let fallback_panel_url = normalize_da_panel_url(state.da_url.as_ref());
    let mut by_domain: HashMap<String, HostingServiceItem> = HashMap::new();

    {
        let hosting_services = state.hosting_services_by_user.read().await;
        if let Some(list) = hosting_services.get(&user) {
            merge_hosting_services(
                &mut by_domain,
                list.iter().cloned().map(|mut item| {
                    backfill_hosting_service_defaults(&mut item, &fallback_panel_url);
                    item
                }),
            );
        }
    }

    {
        let transactions = state.transactions_by_user.read().await;
        if let Some(list) = transactions.get(&user) {
            for mut inferred in collect_hosting_services(list, &fallback_panel_url) {
                backfill_hosting_service_defaults(&mut inferred, &fallback_panel_url);
                let key = inferred.domain.to_ascii_lowercase();
                by_domain.entry(key).or_insert(inferred);
            }
        }
    }

    let mut items: Vec<HostingServiceItem> = by_domain.into_values().collect();
    for item in &mut items {
        backfill_hosting_service_defaults(item, &fallback_panel_url);
    }
    items.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    let total_active = items
        .iter()
        .filter(|item| item.status == SERVICE_STATUS_ACTIVE)
        .count();

    (
        StatusCode::OK,
        Json(HostingServicesResponse {
            ok: true,
            total_active,
            items,
        }),
    )
        .into_response()
}

async fn get_notifications(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<NotificationsQuery>,
) -> impl IntoResponse {
    let user = match require_access_user(&state, &headers) {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let limit = query.limit.unwrap_or(20).min(100);
    let notifications = state.notifications_by_user.read().await;
    let mut items = notifications.get(&user).cloned().unwrap_or_default();
    items.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    let unread_count = items.iter().filter(|item| !item.read).count();
    items.truncate(limit);

    (
        StatusCode::OK,
        Json(NotificationsResponse {
            ok: true,
            items,
            unread_count,
        }),
    )
        .into_response()
}

async fn mark_notifications_read(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match require_access_user(&state, &headers) {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    {
        let mut notifications = state.notifications_by_user.write().await;
        if let Some(items) = notifications.get_mut(&user) {
            for item in items.iter_mut() {
                item.read = true;
            }
        }
    }

    if let Err(err) = persist_topup_state(&state).await {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to persist notifications: {err}"),
            Some("PERSISTENCE_ERROR"),
        );
    }

    (
        StatusCode::OK,
        Json(MarkReadNotificationsResponse {
            ok: true,
            unread_count: 0,
        }),
    )
        .into_response()
}

async fn admin_summary(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(resp) = require_admin_user(&state, &headers) {
        return resp;
    }

    let wallets = state.wallets_by_user.read().await;
    let transactions = state.transactions_by_user.read().await;
    let hosting_services = state.hosting_services_by_user.read().await;
    let notifications = state.notifications_by_user.read().await;

    let mut users = HashSet::new();
    for key in wallets.keys() {
        users.insert(key.to_owned());
    }
    for key in transactions.keys() {
        users.insert(key.to_owned());
    }
    for key in hosting_services.keys() {
        users.insert(key.to_owned());
    }
    for key in notifications.keys() {
        users.insert(key.to_owned());
    }

    let fallback_panel = normalize_da_panel_url(state.da_url.as_ref());
    let mut total_active_services = 0usize;
    let mut total_services_all_status = 0usize;
    for list in hosting_services.values() {
        for item in list {
            let mut item = item.clone();
            backfill_hosting_service_defaults(&mut item, &fallback_panel);
            total_services_all_status += 1;
            if item.status == SERVICE_STATUS_ACTIVE {
                total_active_services += 1;
            }
        }
    }

    let total_transactions = transactions.values().map(|list| list.len()).sum::<usize>();
    let wallet_total_thb = round_money(
        wallets
            .values()
            .map(|wallet| wallet.balance_thb)
            .sum::<f64>(),
    );
    let unread_notifications_total = notifications
        .values()
        .map(|list| list.iter().filter(|item| !item.read).count())
        .sum::<usize>();

    (
        StatusCode::OK,
        Json(AdminSummaryResponse {
            ok: true,
            total_users: users.len(),
            total_active_services,
            total_services_all_status,
            total_transactions,
            wallet_total_thb,
            unread_notifications_total,
        }),
    )
        .into_response()
}

async fn admin_recent_services(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminListQuery>,
) -> impl IntoResponse {
    if let Err(resp) = require_admin_user(&state, &headers) {
        return resp;
    }

    let limit = query.limit.unwrap_or(20).min(200);
    let fallback_panel = normalize_da_panel_url(state.da_url.as_ref());
    let mut items: Vec<AdminServiceView> = Vec::new();

    let hosting_services = state.hosting_services_by_user.read().await;
    for (owner_username, services) in hosting_services.iter() {
        for service in services {
            let mut service = service.clone();
            backfill_hosting_service_defaults(&mut service, &fallback_panel);
            items.push(AdminServiceView {
                owner_username: owner_username.to_owned(),
                domain: service.domain,
                package_name: service.package_name,
                status: service.status,
                created_at: service.created_at,
                expires_at: service.expires_at,
                grace_until: service.grace_until,
                da_username_masked: mask_da_username(service.da_username.as_deref()),
                da_password_masked: mask_da_password(service.da_password.as_deref()),
            });
        }
    }
    items.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    items.truncate(limit);

    (
        StatusCode::OK,
        Json(AdminServicesResponse { ok: true, items }),
    )
        .into_response()
}

async fn admin_recent_transactions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminListQuery>,
) -> impl IntoResponse {
    if let Err(resp) = require_admin_user(&state, &headers) {
        return resp;
    }

    let limit = query.limit.unwrap_or(20).min(200);
    let transactions = state.transactions_by_user.read().await;
    let mut items: Vec<AdminTransactionView> = Vec::new();

    for (owner_username, txs) in transactions.iter() {
        for tx in txs {
            items.push(AdminTransactionView {
                tx_id: tx.tx_id.to_owned(),
                owner_username: owner_username.to_owned(),
                voucher_hash_masked: mask_voucher_hash_admin(&tx.voucher_hash),
                voucher_method: detect_voucher_method(&tx.voucher_hash),
                amount_thb: tx.amount_thb,
                status: tx.status.to_owned(),
                message: tx.message.to_owned(),
                created_at: tx.created_at.to_owned(),
            });
        }
    }
    items.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    items.truncate(limit);

    (
        StatusCode::OK,
        Json(AdminTransactionsResponse { ok: true, items }),
    )
        .into_response()
}

async fn admin_user_wallets(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(resp) = require_admin_user(&state, &headers) {
        return resp;
    }

    let wallets = state.wallets_by_user.read().await;
    let transactions = state.transactions_by_user.read().await;
    let hosting_services = state.hosting_services_by_user.read().await;
    let notifications = state.notifications_by_user.read().await;

    let mut users = HashSet::new();
    for key in wallets.keys() {
        users.insert(key.to_owned());
    }
    for key in transactions.keys() {
        users.insert(key.to_owned());
    }
    for key in hosting_services.keys() {
        users.insert(key.to_owned());
    }
    for key in notifications.keys() {
        users.insert(key.to_owned());
    }

    let mut items: Vec<AdminUserWalletView> = users
        .into_iter()
        .map(|username| AdminUserWalletView {
            balance_thb: round_money(
                wallets
                    .get(&username)
                    .map(|wallet| wallet.balance_thb)
                    .unwrap_or(0.0),
            ),
            username,
        })
        .collect();
    items.sort_by(|left, right| left.username.cmp(&right.username));

    (
        StatusCode::OK,
        Json(AdminUserWalletsResponse { ok: true, items }),
    )
        .into_response()
}

async fn redeem_banking_slip(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<BankingSlipRedeemRequest>,
) -> impl IntoResponse {
    let user = match require_access_user(&state, &headers) {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let img_data_url = payload.img.trim();
    if !is_valid_image_data_url(img_data_url) {
        let tx = build_failed_tx(
            &user,
            "slip:invalid",
            "INVALID_IMAGE_DATA_URL",
            "Invalid slip image format",
        );
        append_transaction(&state, tx).await;
        let _ = persist_topup_state(&state).await;
        return redeem_response(
            StatusCode::BAD_REQUEST,
            false,
            "Invalid slip image format",
            0.0,
            Some("INVALID_IMAGE_DATA_URL".to_owned()),
        );
    }

    let client_ip = normalized_client_ip(&headers);
    if !allow_rate_limit(&state, &format!("{user}|{client_ip}")).await {
        return redeem_response(
            StatusCode::TOO_MANY_REQUESTS,
            false,
            "Too many redeem attempts. Please wait and retry.",
            0.0,
            Some("RATE_LIMITED".to_owned()),
        );
    }

    let slip_data = match verify_slip_image_live(img_data_url, &state).await {
        Ok(data) => data,
        Err((status, code, message)) => {
            let tx = build_failed_tx(&user, "slip:invalid", &code, &message);
            append_transaction(&state, tx).await;
            let _ = persist_topup_state(&state).await;
            return redeem_response(status, false, &message, 0.0, Some(code));
        }
    };

    let slip_ref_raw = slip_data.ref_id.trim();
    if slip_ref_raw.is_empty() {
        let tx = build_failed_tx(
            &user,
            "slip:invalid",
            "SLIP_API_INVALID_RESPONSE",
            "Slip response missing reference id",
        );
        append_transaction(&state, tx).await;
        let _ = persist_topup_state(&state).await;
        return redeem_response(
            StatusCode::BAD_GATEWAY,
            false,
            "Slip response missing reference id",
            0.0,
            Some("SLIP_API_INVALID_RESPONSE".to_owned()),
        );
    }

    let slip_ref_storage = to_slip_storage_reference(slip_ref_raw);
    let slip_ref_key = normalize_slip_ref(slip_ref_raw);
    {
        let used_refs = state.used_slip_refs.read().await;
        if used_refs.contains(&slip_ref_key) {
            let tx = build_failed_tx(
                &user,
                &slip_ref_storage,
                "SLIP_REF_DUPLICATE",
                "Slip reference already used",
            );
            drop(used_refs);
            append_transaction(&state, tx).await;
            let _ = persist_topup_state(&state).await;
            return redeem_response(
                StatusCode::CONFLICT,
                false,
                "Slip reference already used",
                0.0,
                Some("SLIP_REF_DUPLICATE".to_owned()),
            );
        }
    }

    let slip_date = slip_data.date.trim();
    if slip_date.is_empty() {
        let tx = build_failed_tx(
            &user,
            &slip_ref_storage,
            "SLIP_API_INVALID_RESPONSE",
            "Slip date is missing",
        );
        append_transaction(&state, tx).await;
        let _ = persist_topup_state(&state).await;
        return redeem_response(
            StatusCode::BAD_GATEWAY,
            false,
            "Slip date is missing",
            0.0,
            Some("SLIP_API_INVALID_RESPONSE".to_owned()),
        );
    }

    let thai_current_date = match fetch_bangkok_current_date(state.banking_slip_timeout_ms).await {
        Ok(date) => date,
        Err((status, code, message)) => {
            let tx = build_failed_tx(&user, &slip_ref_storage, &code, &message);
            append_transaction(&state, tx).await;
            let _ = persist_topup_state(&state).await;
            return redeem_response(status, false, &message, 0.0, Some(code));
        }
    };

    if !slip_date_matches_date_prefix(slip_date, &thai_current_date) {
        let tx = build_failed_tx(
            &user,
            &slip_ref_storage,
            "SLIP_NOT_TODAY_THAI",
            "Slip date is not today in Thai timezone",
        );
        append_transaction(&state, tx).await;
        let _ = persist_topup_state(&state).await;
        return redeem_response(
            StatusCode::BAD_REQUEST,
            false,
            "Slip date is not today in Thai timezone",
            0.0,
            Some("SLIP_NOT_TODAY_THAI".to_owned()),
        );
    }

    let expected_receiver_name = normalize_receiver_name(state.banking_receiver_name.as_ref());
    let expected_receiver_id = normalize_receiver_id(state.banking_receiver_id.as_ref());
    let actual_receiver_name = normalize_receiver_name(&slip_data.receiver_name);
    let actual_receiver_id = normalize_receiver_id(&slip_data.receiver_id);
    if actual_receiver_name != expected_receiver_name || actual_receiver_id != expected_receiver_id
    {
        let tx = build_failed_tx(
            &user,
            &slip_ref_storage,
            "SLIP_RECEIVER_MISMATCH",
            "Slip receiver does not match configured account",
        );
        append_transaction(&state, tx).await;
        let _ = persist_topup_state(&state).await;
        return redeem_response(
            StatusCode::BAD_REQUEST,
            false,
            "Slip receiver does not match configured account",
            0.0,
            Some("SLIP_RECEIVER_MISMATCH".to_owned()),
        );
    }

    let amount = match parse_slip_amount(&slip_data.amount) {
        Some(value) if value > 0.0 => round_money(value),
        _ => {
            let tx = build_failed_tx(
                &user,
                &slip_ref_storage,
                "SLIP_AMOUNT_INVALID",
                "Slip amount is invalid",
            );
            append_transaction(&state, tx).await;
            let _ = persist_topup_state(&state).await;
            return redeem_response(
                StatusCode::BAD_REQUEST,
                false,
                "Slip amount is invalid",
                0.0,
                Some("SLIP_AMOUNT_INVALID".to_owned()),
            );
        }
    };

    let _commit_guard = state.topup_commit_lock.lock().await;
    {
        let used_refs = state.used_slip_refs.read().await;
        if used_refs.contains(&slip_ref_key) {
            return redeem_response(
                StatusCode::CONFLICT,
                false,
                "Slip reference already used",
                0.0,
                Some("SLIP_REF_DUPLICATE".to_owned()),
            );
        }
    }

    {
        let mut used_refs = state.used_slip_refs.write().await;
        used_refs.insert(slip_ref_key);
    }

    {
        let mut wallets = state.wallets_by_user.write().await;
        let entry = wallets
            .entry(user.to_owned())
            .or_insert_with(|| WalletAccount {
                username: user.to_owned(),
                balance_thb: 0.0,
                updated_at: now_iso(),
            });

        entry.balance_thb = round_money(entry.balance_thb + amount);
        entry.updated_at = now_iso();
    }

    append_transaction(
        &state,
        TopupTransaction {
            tx_id: Uuid::new_v4().to_string(),
            username: user,
            voucher_hash: slip_ref_storage,
            amount_thb: amount,
            status: "success".to_owned(),
            error_code: None,
            message: "Banking slip verified and topped up".to_owned(),
            created_at: now_iso(),
        },
    )
    .await;

    if let Err(err) = persist_topup_state(&state).await {
        return redeem_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            false,
            &format!("failed to persist topup state: {err}"),
            0.0,
            Some("PERSISTENCE_ERROR".to_owned()),
        );
    }

    redeem_response(
        StatusCode::OK,
        true,
        "Banking slip verified and topped up",
        amount,
        None,
    )
}

async fn redeem_truemoney(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RedeemRequest>,
) -> impl IntoResponse {
    let user = match require_access_user(&state, &headers) {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    if payload.voucher_url.len() > 512 {
        let tx = build_failed_tx(
            &user,
            "invalid",
            "INVALID_VOUCHER_URL",
            "Voucher URL is too long",
        );
        append_transaction(&state, tx).await;
        let _ = persist_topup_state(&state).await;
        return redeem_response(
            StatusCode::BAD_REQUEST,
            false,
            "Voucher URL is too long",
            0.0,
            Some("INVALID_VOUCHER_URL".to_owned()),
        );
    }

    let client_ip = normalized_client_ip(&headers);
    if !allow_rate_limit(&state, &format!("{user}|{client_ip}")).await {
        return redeem_response(
            StatusCode::TOO_MANY_REQUESTS,
            false,
            "Too many redeem attempts. Please wait and retry.",
            0.0,
            Some("RATE_LIMITED".to_owned()),
        );
    }

    let voucher_hash = match extract_voucher_hash(&payload.voucher_url) {
        Ok(hash) => hash.to_ascii_lowercase(),
        Err(code) => {
            let tx = build_failed_tx(&user, "invalid", code, "Invalid TrueMoney voucher URL");
            append_transaction(&state, tx).await;
            let _ = persist_topup_state(&state).await;
            return redeem_response(
                StatusCode::BAD_REQUEST,
                false,
                "Invalid TrueMoney voucher URL",
                0.0,
                Some(code.to_owned()),
            );
        }
    };

    {
        let redeemed = state.redeemed_vouchers.read().await;
        if redeemed.contains(&voucher_hash) {
            let tx = build_failed_tx(
                &user,
                &voucher_hash,
                "VOUCHER_ALREADY_REDEEMED",
                "Voucher already redeemed",
            );
            drop(redeemed);
            append_transaction(&state, tx).await;
            let _ = persist_topup_state(&state).await;
            return redeem_response(
                StatusCode::CONFLICT,
                false,
                "Voucher already redeemed",
                0.0,
                Some("VOUCHER_ALREADY_REDEEMED".to_owned()),
            );
        }
    }

    let outcome = redeem_voucher_live(
        &voucher_hash,
        state.receiver_phone.as_ref(),
        state.truemoney_timeout_ms,
    )
    .await;

    if !outcome.success {
        let error_code = outcome
            .error_code
            .as_ref()
            .map(|code| code.as_str())
            .unwrap_or("REDEEM_FAILED");
        let tx = build_failed_tx(&user, &voucher_hash, error_code, &outcome.message);
        append_transaction(&state, tx).await;
        let _ = persist_topup_state(&state).await;
        return redeem_response(
            StatusCode::BAD_REQUEST,
            false,
            &outcome.message,
            0.0,
            outcome.error_code,
        );
    }

    let _commit_guard = state.topup_commit_lock.lock().await;
    {
        let redeemed = state.redeemed_vouchers.read().await;
        if redeemed.contains(&voucher_hash) {
            return redeem_response(
                StatusCode::CONFLICT,
                false,
                "Voucher already redeemed",
                0.0,
                Some("VOUCHER_ALREADY_REDEEMED".to_owned()),
            );
        }
    }

    let rounded_amount = round_money(outcome.amount_thb);

    {
        let mut redeemed = state.redeemed_vouchers.write().await;
        redeemed.insert(voucher_hash.to_owned());
    }

    let _new_balance = {
        let mut wallets = state.wallets_by_user.write().await;
        let entry = wallets
            .entry(user.to_owned())
            .or_insert_with(|| WalletAccount {
                username: user.to_owned(),
                balance_thb: 0.0,
                updated_at: now_iso(),
            });

        entry.balance_thb = round_money(entry.balance_thb + rounded_amount);
        entry.updated_at = now_iso();
        entry.balance_thb
    };

    append_transaction(
        &state,
        TopupTransaction {
            tx_id: Uuid::new_v4().to_string(),
            username: user,
            voucher_hash,
            amount_thb: rounded_amount,
            status: "success".to_owned(),
            error_code: None,
            message: outcome.message.to_owned(),
            created_at: now_iso(),
        },
    )
    .await;

    if let Err(err) = persist_topup_state(&state).await {
        return redeem_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            false,
            &format!("failed to persist topup state: {err}"),
            0.0,
            Some("PERSISTENCE_ERROR".to_owned()),
        );
    }

    redeem_response(StatusCode::OK, true, &outcome.message, rounded_amount, None)
}

fn resolve_da_package(package_name: &str) -> Option<&'static str> {
    match package_name {
        "Start" => Some("200mb"),
        "Lite" => Some("500mb"),
        "Core" => Some("1gb"),
        "Plus" => Some("2gb"),
        "Prime" => Some("5gb"),
        "Pro" => Some("10gb"),
        "Max" => Some("15gb"),
        "Apex" => Some("20gb"),
        _ => None,
    }
}

fn generate_da_username(domain: &str) -> String {
    let prefix: String = domain
        .chars()
        .filter(|ch| ch.is_ascii_lowercase())
        .take(4)
        .collect();
    let suffix = &Uuid::new_v4().to_string().replace('-', "")[..4];
    let name = if prefix.is_empty() {
        format!("rv{suffix}")
    } else {
        format!("{prefix}{suffix}")
    };
    name[..name.len().min(10)].to_owned()
}

fn generate_da_password() -> String {
    let id = Uuid::new_v4().to_string().replace('-', "");
    format!("Rv{}!", &id[..12])
}

async fn create_hosting_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateHostingOrderRequest>,
) -> impl IntoResponse {
    let user = match require_access_user(&state, &headers) {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let domain = payload.domain.trim().to_owned();
    let email = payload.email.trim().to_owned();
    let package_name = payload.package_name.trim().to_owned();
    let price = payload.price;

    if domain.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Domain is required",
            Some("MISSING_DOMAIN"),
        );
    }

    if email.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Email is required",
            Some("MISSING_EMAIL"),
        );
    }

    if price <= 0.0 {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Invalid price",
            Some("INVALID_PRICE"),
        );
    }

    let da_package = match resolve_da_package(&package_name) {
        Some(pkg) => pkg,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Invalid package",
                Some("INVALID_PACKAGE"),
            )
        }
    };

    if state.da_username.is_empty() || state.da_password.is_empty() {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DirectAdmin credentials are not configured",
            Some("DA_NOT_CONFIGURED"),
        );
    }

    let _lock = state.topup_commit_lock.lock().await;

    let current_balance = {
        let wallets = state.wallets_by_user.read().await;
        wallets.get(&user).map(|w| w.balance_thb).unwrap_or(0.0)
    };

    if current_balance < price {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Insufficient balance",
            Some("INSUFFICIENT_BALANCE"),
        );
    }

    {
        let mut wallets = state.wallets_by_user.write().await;
        let wallet = wallets
            .entry(user.to_owned())
            .or_insert_with(|| WalletAccount {
                username: user.to_owned(),
                balance_thb: 0.0,
                updated_at: now_iso(),
            });
        wallet.balance_thb = round_money(wallet.balance_thb - price);
        wallet.updated_at = now_iso();
    }

    let da_user = generate_da_username(&domain);
    let da_pass = generate_da_password();

    let da_result =
        call_directadmin_create_user(&state, &da_user, &email, &da_pass, &domain, da_package).await;

    if let Err(ref err_msg) = da_result {
        eprintln!("[create_hosting_order] DirectAdmin error: {err_msg}");

        {
            let mut wallets = state.wallets_by_user.write().await;
            if let Some(wallet) = wallets.get_mut(&user) {
                wallet.balance_thb = round_money(wallet.balance_thb + price);
                wallet.updated_at = now_iso();
            }
        }

        let _ = persist_topup_state(&state).await;

        return error_response(
            StatusCode::BAD_GATEWAY,
            &format!("Failed to provision hosting: {err_msg}"),
            Some("DA_PROVISION_ERROR"),
        );
    }

    let created_at = now_iso();
    let expires_at =
        add_calendar_month_to_iso(&created_at).unwrap_or_else(|| created_at.to_owned());
    let grace_until = add_days_to_iso(&expires_at, 1).unwrap_or_else(|| expires_at.to_owned());
    let panel_url = normalize_da_panel_url(state.da_url.as_ref());

    let tx = TopupTransaction {
        tx_id: Uuid::new_v4().to_string(),
        username: user.to_owned(),
        voucher_hash: format!("order:{}:{}", package_name, domain),
        amount_thb: -price,
        status: "success".to_owned(),
        error_code: None,
        message: format!("Hosting order: {} ({})", package_name, domain),
        created_at: created_at.to_owned(),
    };
    append_transaction(&state, tx).await;

    upsert_hosting_service_item(
        &state,
        &user,
        HostingServiceItem {
            domain: domain.to_owned(),
            package_name: package_name.to_owned(),
            created_at: created_at.to_owned(),
            status: SERVICE_STATUS_ACTIVE.to_owned(),
            expires_at: expires_at.to_owned(),
            grace_until: grace_until.to_owned(),
            suspended_at: None,
            billing_price_thb: round_money(price),
            notified_d1_at: None,
            notified_expired_at: None,
            notified_grace_end_at: None,
            da_username: Some(da_user.to_owned()),
            da_password: Some(da_pass.to_owned()),
            da_panel_url: Some(panel_url.to_owned()),
        },
    )
    .await;
    push_notification(
        &state,
        &user,
        "service_created",
        "Hosting service created",
        &format!(
            "Service for {} is active. DirectAdmin panel: {}",
            domain, panel_url
        ),
    )
    .await;

    if let Err(err) = persist_topup_state(&state).await {
        eprintln!("[create_hosting_order] persist error: {err}");
    }

    (
        StatusCode::OK,
        Json(CreateHostingOrderResponse {
            ok: true,
            message: format!("Hosting provisioned successfully for {domain}"),
            da_username: Some(da_user),
            da_password: Some(da_pass),
            da_panel_url: Some(panel_url),
        }),
    )
        .into_response()
}

async fn renew_hosting_service(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RenewHostingServiceRequest>,
) -> impl IntoResponse {
    let user = match require_access_user(&state, &headers) {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    let domain = payload.domain.trim().to_owned();
    if domain.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Domain is required",
            Some("MISSING_DOMAIN"),
        );
    }

    let _lock = state.topup_commit_lock.lock().await;
    let now = Utc::now();
    let fallback_panel = normalize_da_panel_url(state.da_url.as_ref());

    let mut service_snapshot = {
        let hosting_services = state.hosting_services_by_user.read().await;
        hosting_services.get(&user).and_then(|items| {
            items
                .iter()
                .find(|item| item.domain.eq_ignore_ascii_case(&domain))
                .cloned()
        })
    };

    let Some(mut service) = service_snapshot.take() else {
        return error_response(
            StatusCode::NOT_FOUND,
            "Service domain not found",
            Some("SERVICE_NOT_FOUND"),
        );
    };
    backfill_hosting_service_defaults(&mut service, &fallback_panel);

    let can_renew = is_renew_allowed(&service.status, &service.grace_until, now);

    if !can_renew {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Renewal window has expired",
            Some("RENEWAL_WINDOW_EXPIRED"),
        );
    }

    let charged_amount = if service.billing_price_thb > 0.0 {
        round_money(service.billing_price_thb)
    } else {
        round_money(resolve_package_price(&service.package_name).unwrap_or(0.0))
    };
    if charged_amount <= 0.0 {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Unable to determine renewal price",
            Some("INVALID_RENEWAL_PRICE"),
        );
    }

    let current_balance = {
        let wallets = state.wallets_by_user.read().await;
        wallets
            .get(&user)
            .map(|wallet| wallet.balance_thb)
            .unwrap_or(0.0)
    };
    if current_balance < charged_amount {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Insufficient balance",
            Some("INSUFFICIENT_BALANCE"),
        );
    }

    if service.status == SERVICE_STATUS_GRACE_SUSPENDED {
        let Some(ref da_username) = service.da_username else {
            return error_response(
                StatusCode::BAD_GATEWAY,
                "Service is suspended but missing DirectAdmin username",
                Some("DA_USERNAME_MISSING"),
            );
        };
        if let Err(err) = call_directadmin_set_suspension(&state, da_username, false).await {
            eprintln!("[renew_hosting_service] failed to unsuspend: {err}");
            return error_response(
                StatusCode::BAD_GATEWAY,
                &format!("DirectAdmin unsuspend failed: {err}"),
                Some("DA_UNSUSPEND_ERROR"),
            );
        }
    }

    let new_expires_at = add_calendar_month_to_iso(&service.expires_at)
        .or_else(|| add_calendar_month_to_iso(&service.created_at))
        .unwrap_or_else(now_iso);
    let new_grace_until =
        add_days_to_iso(&new_expires_at, 1).unwrap_or_else(|| new_expires_at.to_owned());

    let balance_thb = {
        let mut wallets = state.wallets_by_user.write().await;
        let wallet = wallets
            .entry(user.to_owned())
            .or_insert_with(|| WalletAccount {
                username: user.to_owned(),
                balance_thb: 0.0,
                updated_at: now_iso(),
            });
        wallet.balance_thb = round_money(wallet.balance_thb - charged_amount);
        wallet.updated_at = now_iso();
        wallet.balance_thb
    };

    {
        let mut hosting_services = state.hosting_services_by_user.write().await;
        let Some(items) = hosting_services.get_mut(&user) else {
            return error_response(
                StatusCode::NOT_FOUND,
                "Service domain not found",
                Some("SERVICE_NOT_FOUND"),
            );
        };
        let Some(item) = items
            .iter_mut()
            .find(|item| item.domain.eq_ignore_ascii_case(&domain))
        else {
            return error_response(
                StatusCode::NOT_FOUND,
                "Service domain not found",
                Some("SERVICE_NOT_FOUND"),
            );
        };

        item.status = SERVICE_STATUS_ACTIVE.to_owned();
        item.expires_at = new_expires_at.to_owned();
        item.grace_until = new_grace_until.to_owned();
        item.suspended_at = None;
        item.notified_d1_at = None;
        item.notified_expired_at = None;
        item.notified_grace_end_at = None;
        item.billing_price_thb = charged_amount;
        if item
            .da_panel_url
            .as_deref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true)
        {
            item.da_panel_url = Some(fallback_panel.to_owned());
        }
    }

    let tx = TopupTransaction {
        tx_id: Uuid::new_v4().to_string(),
        username: user.to_owned(),
        voucher_hash: format!("order:{}:{}", service.package_name, service.domain),
        amount_thb: -charged_amount,
        status: "success".to_owned(),
        error_code: None,
        message: format!(
            "Hosting renewal: {} ({})",
            service.package_name, service.domain
        ),
        created_at: now_iso(),
    };
    append_transaction(&state, tx).await;
    push_notification(
        &state,
        &user,
        "service_renewed",
        "Hosting service renewed",
        &format!("{} renewed until {}", service.domain, new_expires_at),
    )
    .await;

    if let Err(err) = persist_topup_state(&state).await {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to persist renewal: {err}"),
            Some("PERSISTENCE_ERROR"),
        );
    }

    (
        StatusCode::OK,
        Json(RenewHostingServiceResponse {
            ok: true,
            message: format!("Renewed {} for 1 month", service.domain),
            domain: service.domain,
            status: SERVICE_STATUS_ACTIVE.to_owned(),
            expires_at: new_expires_at,
            grace_until: new_grace_until,
            charged_amount,
            balance_thb,
        }),
    )
        .into_response()
}

async fn call_directadmin_create_user(
    state: &AppState,
    username: &str,
    email: &str,
    password: &str,
    domain: &str,
    package: &str,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| format!("Failed to build HTTP client: {err}"))?;

    let url = format!("{}/CMD_API_ACCOUNT_USER", state.da_url);

    let params = [
        ("action", "create"),
        ("add", "Submit"),
        ("username", username),
        ("email", email),
        ("passwd", password),
        ("passwd2", password),
        ("domain", domain),
        ("package", package),
        ("ip", state.da_server_ip.as_ref()),
        ("notify", "no"),
    ];

    let response = client
        .post(&url)
        .basic_auth(state.da_username.as_ref(), Some(state.da_password.as_ref()))
        .form(&params)
        .send()
        .await
        .map_err(|err| format!("DirectAdmin request failed: {err}"))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_else(|_| String::new());

    eprintln!(
        "[call_directadmin_create_user] status={} body={}",
        status.as_u16(),
        body
    );

    if !status.is_success() {
        return Err(format!(
            "DirectAdmin returned HTTP {}: {}",
            status.as_u16(),
            body
        ));
    }

    if body.contains("error=1") {
        return Err(format!(
            "DirectAdmin error: {}",
            extract_directadmin_error_message(&body)
        ));
    }

    Ok(())
}

async fn call_directadmin_set_suspension(
    state: &AppState,
    da_username: &str,
    suspended: bool,
) -> Result<(), String> {
    let username = da_username.trim();
    if username.is_empty() {
        return Err("DirectAdmin username is empty".to_owned());
    }
    if state.da_username.is_empty() || state.da_password.is_empty() {
        return Err("DirectAdmin credentials are not configured".to_owned());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| format!("Failed to build HTTP client: {err}"))?;

    let url = format!(
        "{}/CMD_API_SELECT_USERS",
        state.da_url.as_ref().trim_end_matches('/')
    );
    let forms: Vec<Vec<(&str, &str)>> = if suspended {
        vec![
            vec![
                ("location", "CMD_SELECT_USERS"),
                ("select0", username),
                ("dosuspend", "yes"),
            ],
            vec![
                ("location", "CMD_SELECT_USERS"),
                ("select0", username),
                ("suspend", "Suspend"),
            ],
            vec![("action", "suspend"), ("username", username)],
        ]
    } else {
        vec![
            vec![
                ("location", "CMD_SELECT_USERS"),
                ("select0", username),
                ("dounsuspend", "yes"),
            ],
            vec![
                ("location", "CMD_SELECT_USERS"),
                ("select0", username),
                ("unsuspend", "Unsuspend"),
            ],
            vec![("action", "unsuspend"), ("username", username)],
        ]
    };

    let mut last_err = "Unknown DirectAdmin suspend error".to_owned();
    for params in forms {
        let response = client
            .post(&url)
            .basic_auth(state.da_username.as_ref(), Some(state.da_password.as_ref()))
            .form(&params)
            .send()
            .await
            .map_err(|err| format!("DirectAdmin request failed: {err}"))?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        eprintln!(
            "[call_directadmin_set_suspension] action={} user={} status={} body={}",
            if suspended { "suspend" } else { "unsuspend" },
            username,
            status.as_u16(),
            body
        );

        if !status.is_success() {
            last_err = format!("DirectAdmin returned HTTP {}", status.as_u16());
            continue;
        }
        if body.contains("error=1") {
            let parsed_error = extract_directadmin_error_message(&body);
            last_err = format!("DirectAdmin error: {parsed_error}");
            continue;
        }

        return Ok(());
    }

    Err(last_err)
}

fn build_failed_tx(user: &str, voucher_hash: &str, code: &str, message: &str) -> TopupTransaction {
    TopupTransaction {
        tx_id: Uuid::new_v4().to_string(),
        username: user.to_owned(),
        voucher_hash: voucher_hash.to_owned(),
        amount_thb: 0.0,
        status: "failed".to_owned(),
        error_code: Some(code.to_owned()),
        message: message.to_owned(),
        created_at: now_iso(),
    }
}

async fn append_transaction(state: &AppState, transaction: TopupTransaction) {
    let key = transaction.username.to_owned();
    let mut map = state.transactions_by_user.write().await;
    let list = map.entry(key).or_default();
    list.push(transaction);
}

async fn upsert_hosting_service_item(state: &AppState, username: &str, item: HostingServiceItem) {
    let mut map = state.hosting_services_by_user.write().await;
    let list = map.entry(username.to_owned()).or_default();

    if let Some(existing) = list
        .iter_mut()
        .find(|existing| existing.domain.eq_ignore_ascii_case(&item.domain))
    {
        if existing.created_at <= item.created_at {
            *existing = item;
        }
        return;
    }

    list.push(item);
}

async fn persist_sessions(state: &AppState) -> Result<(), String> {
    let _guard = state.persistence_lock.lock().await;
    let sessions = state.sessions_by_user.read().await;
    save_json_to_file_atomic(state.sessions_storage_file.as_ref(), &*sessions)
}

async fn persist_refresh_sessions(state: &AppState) -> Result<(), String> {
    let _guard = state.persistence_lock.lock().await;
    let sessions = state.refresh_sessions.read().await;
    save_json_to_file_atomic(state.refresh_sessions_storage_file.as_ref(), &*sessions)
}

async fn persist_topup_state(state: &AppState) -> Result<(), String> {
    let _guard = state.persistence_lock.lock().await;

    let wallets = state.wallets_by_user.read().await;
    save_json_to_file_atomic(state.wallets_storage_file.as_ref(), &*wallets)?;
    drop(wallets);

    let transactions = state.transactions_by_user.read().await;
    save_json_to_file_atomic(state.transactions_storage_file.as_ref(), &*transactions)?;
    drop(transactions);

    let hosting_services = state.hosting_services_by_user.read().await;
    save_json_to_file_atomic(
        state.hosting_services_storage_file.as_ref(),
        &*hosting_services,
    )?;
    drop(hosting_services);

    let notifications = state.notifications_by_user.read().await;
    save_json_to_file_atomic(state.notifications_storage_file.as_ref(), &*notifications)?;
    drop(notifications);

    let redeemed = state.redeemed_vouchers.read().await;
    save_json_to_file_atomic(state.redeemed_storage_file.as_ref(), &*redeemed)?;
    drop(redeemed);

    let used_refs = state.used_slip_refs.read().await;
    save_json_to_file_atomic(state.used_slip_refs_storage_file.as_ref(), &*used_refs)?;

    Ok(())
}

fn revoke_refresh_family(
    sessions: &mut HashMap<String, RefreshSession>,
    family_id: &str,
    now_unix: u64,
) {
    for session in sessions.values_mut() {
        if session.family_id == family_id {
            session.revoked_at_unix = Some(now_unix);
        }
    }
}

fn revoke_refresh_user(
    sessions: &mut HashMap<String, RefreshSession>,
    username: &str,
    now_unix: u64,
) {
    for session in sessions.values_mut() {
        if session.username == username {
            session.revoked_at_unix = Some(now_unix);
        }
    }
}

fn require_access_principal(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(String, UserRole), Response> {
    let token = match extract_bearer_token(headers) {
        Ok(token) => token,
        Err(AuthError::MissingToken) => {
            return Err(error_response(
                StatusCode::UNAUTHORIZED,
                "Missing Bearer token",
                Some("UNAUTHENTICATED"),
            ))
        }
        Err(AuthError::InvalidToken) => {
            return Err(error_response(
                StatusCode::UNAUTHORIZED,
                "Invalid or expired token",
                Some("INVALID_ACCESS_TOKEN"),
            ))
        }
    };

    let principal = validate_access_token(token, state.auth_config.as_ref()).map_err(|_| {
        error_response(
            StatusCode::UNAUTHORIZED,
            "Invalid or expired token",
            Some("INVALID_ACCESS_TOKEN"),
        )
    })?;

    Ok((principal.username, principal.role))
}

fn require_access_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let (username, role) = require_access_principal(state, headers)?;
    if role != UserRole::Root {
        return Err(error_response(
            StatusCode::FORBIDDEN,
            "Root role required",
            Some("FORBIDDEN_ROLE"),
        ));
    }
    Ok(username)
}

fn require_admin_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let (username, role) = require_access_principal(state, headers)?;
    if role != UserRole::Admin {
        return Err(error_response(
            StatusCode::FORBIDDEN,
            "Admin role required",
            Some("FORBIDDEN_ROLE"),
        ));
    }
    Ok(username)
}

fn require_any_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let (username, _) = require_access_principal(state, headers)?;
    Ok(username)
}

fn error_response(status: StatusCode, message: &str, error_code: Option<&str>) -> Response {
    log_error("error_response", status, error_code, message);
    (
        status,
        Json(ErrorResponse {
            ok: false,
            message: message.to_owned(),
            error_code: error_code.map(|code| code.to_owned()),
        }),
    )
        .into_response()
}

fn redeem_response(
    status: StatusCode,
    success: bool,
    message: &str,
    amount: f64,
    error_code: Option<String>,
) -> Response {
    if !success || status.is_client_error() || status.is_server_error() {
        log_error("redeem_response", status, error_code.as_deref(), message);
    }
    (
        status,
        Json(RedeemResponse {
            success,
            amount,
            message: message.to_owned(),
            error_code,
        }),
    )
        .into_response()
}

fn json_with_cookie<T>(status: StatusCode, payload: T, cookie: &str) -> Response
where
    T: serde::Serialize,
{
    let mut response = (status, Json(payload)).into_response();
    if let Ok(header) = HeaderValue::from_str(cookie) {
        response.headers_mut().append(SET_COOKIE, header);
    }
    response
}

fn log_error(context: &str, status: StatusCode, error_code: Option<&str>, message: &str) {
    let code = error_code.unwrap_or("-");
    eprintln!(
        "[{}] status={} error_code={} message={}",
        context,
        status.as_u16(),
        code,
        message
    );
}

async fn verify_slip_image_live(
    img_data_url: &str,
    state: &AppState,
) -> Result<SlipApiResponseData, (StatusCode, String, String)> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(state.banking_slip_timeout_ms))
        .build()
        .map_err(|err| {
            eprintln!("[verify_slip_image_live] failed to build client: {err}");
            (
                StatusCode::BAD_GATEWAY,
                "SLIP_API_ERROR".to_owned(),
                format!("Failed to initialize slip API client: {err}"),
            )
        })?;

    let response = client
        .post(state.banking_slip_api_url.as_ref())
        .header("cache-control", "no-store")
        .header("pragma", "no-cache")
        .json(&SlipApiRequest {
            img: img_data_url,
            tos: true,
            privacy: true,
            eula: true,
        })
        .send()
        .await
        .map_err(|err| {
            eprintln!("[verify_slip_image_live] upstream request error: {err}");
            (
                StatusCode::BAD_GATEWAY,
                "SLIP_API_ERROR".to_owned(),
                format!("Failed to verify slip: {err}"),
            )
        })?;

    let upstream_status = response.status();
    let response_text = response.text().await.map_err(|err| {
        eprintln!("[verify_slip_image_live] failed reading upstream body: {err}");
        (
            StatusCode::BAD_GATEWAY,
            "SLIP_API_ERROR".to_owned(),
            format!("Failed to read upstream response: {err}"),
        )
    })?;

    if !upstream_status.is_success() {
        let message = if response_text.trim().is_empty() {
            format!("Slip API returned status {}", upstream_status.as_u16())
        } else {
            response_text
        };
        eprintln!(
            "[verify_slip_image_live] upstream non-success status={} body={}",
            upstream_status.as_u16(),
            message
        );
        return Err((
            StatusCode::from_u16(upstream_status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
            "SLIP_API_ERROR".to_owned(),
            message,
        ));
    }

    eprintln!(
        "[verify_slip_image_live] upstream success status={} body={}",
        upstream_status.as_u16(),
        response_text
    );

    serde_json::from_str::<SlipApiResponse>(&response_text)
        .map(|payload| payload.data)
        .map_err(|err| {
            eprintln!(
                "[verify_slip_image_live] invalid upstream payload parse_error={} body={}",
                err, response_text
            );
            let message = if response_text.trim().is_empty() {
                format!("Slip API response format is invalid: {err}")
            } else {
                response_text
            };
            (
                StatusCode::BAD_GATEWAY,
                "SLIP_API_INVALID_RESPONSE".to_owned(),
                message,
            )
        })
}

fn display_reference_id(stored_reference: &str) -> String {
    if let Some(reference) = stored_reference.strip_prefix("slip:") {
        return reference.to_owned();
    }

    mask_voucher_hash(stored_reference)
}

fn mask_secret_keep_ends(value: &str, prefix_len: usize, suffix_len: usize) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "-".to_owned();
    }

    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= prefix_len + suffix_len {
        return "*".repeat(chars.len().max(4));
    }

    let prefix = chars.iter().take(prefix_len).collect::<String>();
    let suffix = chars
        .iter()
        .skip(chars.len().saturating_sub(suffix_len))
        .collect::<String>();
    format!("{prefix}******{suffix}")
}

fn mask_da_username(value: Option<&str>) -> String {
    match value {
        Some(raw) if !raw.trim().is_empty() => {
            let prefix: String = raw.trim().chars().take(4).collect();
            format!("{prefix}****")
        }
        _ => "-".to_owned(),
    }
}

fn mask_da_password(value: Option<&str>) -> String {
    match value {
        Some(raw) if !raw.trim().is_empty() => mask_secret_keep_ends(raw, 4, 4),
        _ => "-".to_owned(),
    }
}

fn mask_voucher_hash_admin(voucher_hash: &str) -> String {
    if let Some((package, domain)) = parse_hosting_order_reference(voucher_hash) {
        let prefix: String = domain.chars().take(3).collect();
        return format!("order:{package}:{prefix}***");
    }
    if voucher_hash.starts_with("slip:") {
        return "slip:***".to_owned();
    }
    mask_voucher_hash(voucher_hash)
}

fn detect_voucher_method(voucher_hash: &str) -> String {
    if parse_hosting_order_reference(voucher_hash).is_some() {
        return "hosting_order".to_owned();
    }
    if voucher_hash.starts_with("slip:") {
        return "banking".to_owned();
    }
    "truemoney".to_owned()
}

fn parse_hosting_order_reference(stored_reference: &str) -> Option<(String, String)> {
    let raw = stored_reference.strip_prefix("order:")?;
    let mut parts = raw.splitn(2, ':');
    let package_name = parts.next()?.trim();
    let domain = parts.next()?.trim();

    if package_name.is_empty() || domain.is_empty() {
        return None;
    }

    Some((package_name.to_owned(), domain.to_owned()))
}

fn merge_hosting_services<I>(by_domain: &mut HashMap<String, HostingServiceItem>, items: I)
where
    I: IntoIterator<Item = HostingServiceItem>,
{
    for item in items {
        let key = item.domain.to_ascii_lowercase();
        match by_domain.get(&key) {
            Some(existing) if existing.created_at >= item.created_at => {}
            _ => {
                by_domain.insert(key, item);
            }
        }
    }
}

fn collect_hosting_services(
    transactions: &[TopupTransaction],
    fallback_panel_url: &str,
) -> Vec<HostingServiceItem> {
    let mut by_domain: HashMap<String, HostingServiceItem> = HashMap::new();

    for item in transactions.iter().filter(|item| item.status == "success") {
        let Some((package_name, domain)) = parse_hosting_order_reference(&item.voucher_hash) else {
            continue;
        };

        let mut candidate = HostingServiceItem {
            domain,
            package_name: package_name.to_owned(),
            created_at: item.created_at.to_owned(),
            status: SERVICE_STATUS_ACTIVE.to_owned(),
            expires_at: add_calendar_month_to_iso(&item.created_at)
                .unwrap_or_else(|| item.created_at.to_owned()),
            grace_until: add_days_to_iso(
                &add_calendar_month_to_iso(&item.created_at)
                    .unwrap_or_else(|| item.created_at.to_owned()),
                1,
            )
            .unwrap_or_else(|| item.created_at.to_owned()),
            suspended_at: None,
            billing_price_thb: resolve_package_price(&package_name)
                .unwrap_or_else(|| item.amount_thb.abs()),
            notified_d1_at: None,
            notified_expired_at: None,
            notified_grace_end_at: None,
            da_username: None,
            da_password: None,
            da_panel_url: None,
        };
        backfill_hosting_service_defaults(&mut candidate, fallback_panel_url);
        merge_hosting_services(&mut by_domain, [candidate]);
    }

    let mut items: Vec<HostingServiceItem> = by_domain.into_values().collect();
    items.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    items
}

async fn lifecycle_worker(state: AppState) {
    loop {
        if let Err(err) = run_lifecycle_tick(&state).await {
            eprintln!("[lifecycle_worker] tick failed: {err}");
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

async fn run_lifecycle_tick(state: &AppState) -> Result<(), String> {
    let fallback_panel = normalize_da_panel_url(state.da_url.as_ref());
    let now = Utc::now();

    let mut d1_events: Vec<(String, String)> = Vec::new();
    let mut grace_end_events: Vec<(String, String)> = Vec::new();
    {
        let mut services = state.hosting_services_by_user.write().await;
        for (username, items) in services.iter_mut() {
            for item in items.iter_mut() {
                backfill_hosting_service_defaults(item, &fallback_panel);
                let expires_at = parse_iso_datetime(&item.expires_at);
                let grace_until = parse_iso_datetime(&item.grace_until);

                if item.status == SERVICE_STATUS_ACTIVE {
                    if let Some(expires_at) = expires_at {
                        let remaining = expires_at.signed_duration_since(now);
                        if remaining <= ChronoDuration::hours(24)
                            && remaining > ChronoDuration::zero()
                            && item.notified_d1_at.is_none()
                        {
                            item.notified_d1_at = Some(now_iso());
                            d1_events.push((username.to_owned(), item.domain.to_owned()));
                        }
                    }
                }

                if item.status == SERVICE_STATUS_GRACE_SUSPENDED {
                    if let Some(grace_until) = grace_until {
                        if now > grace_until && item.status != SERVICE_STATUS_SUSPENDED_EXPIRED {
                            item.status = SERVICE_STATUS_SUSPENDED_EXPIRED.to_owned();
                            if item.notified_grace_end_at.is_none() {
                                item.notified_grace_end_at = Some(now_iso());
                                grace_end_events
                                    .push((username.to_owned(), item.domain.to_owned()));
                            }
                        }
                    }
                }
            }
        }
    }

    for (username, domain) in d1_events {
        push_notification(
            state,
            &username,
            "service_d1",
            "Hosting service expires in 1 day",
            &format!("{domain} will expire within 24 hours"),
        )
        .await;
    }
    for (username, domain) in grace_end_events {
        push_notification(
            state,
            &username,
            "grace_ended",
            "Grace period ended",
            &format!("{domain} is now suspended. Renewal period ended."),
        )
        .await;
    }

    let mut expire_candidates: Vec<(String, String, String)> = Vec::new();
    {
        let services = state.hosting_services_by_user.read().await;
        for (username, items) in services.iter() {
            for item in items {
                if item.status != SERVICE_STATUS_ACTIVE {
                    continue;
                }
                let Some(expires_at) = parse_iso_datetime(&item.expires_at) else {
                    continue;
                };
                if now < expires_at {
                    continue;
                }
                let Some(da_username) = item.da_username.clone() else {
                    continue;
                };
                expire_candidates.push((username.to_owned(), item.domain.to_owned(), da_username));
            }
        }
    }

    for (username, domain, da_username) in expire_candidates {
        if let Err(err) = call_directadmin_set_suspension(state, &da_username, true).await {
            eprintln!(
                "[lifecycle_worker] suspend failed for user={} domain={} error={}",
                username, domain, err
            );
            continue;
        }

        let mut changed = false;
        {
            let mut services = state.hosting_services_by_user.write().await;
            if let Some(items) = services.get_mut(&username) {
                if let Some(item) = items
                    .iter_mut()
                    .find(|item| item.domain.eq_ignore_ascii_case(&domain))
                {
                    if item.status == SERVICE_STATUS_ACTIVE {
                        item.status = SERVICE_STATUS_GRACE_SUSPENDED.to_owned();
                        item.suspended_at = Some(now_iso());
                        if item.notified_expired_at.is_none() {
                            item.notified_expired_at = Some(now_iso());
                        }
                        changed = true;
                    }
                }
            }
        }

        if changed {
            push_notification(
                state,
                &username,
                "service_expired",
                "Hosting service expired",
                &format!("{domain} expired and has been suspended. You can renew within 1 day."),
            )
            .await;
        }
    }

    persist_topup_state(state).await?;
    Ok(())
}

fn resolve_package_price(package_name: &str) -> Option<f64> {
    match package_name.trim().to_ascii_lowercase().as_str() {
        "start" => Some(10.0),
        "lite" => Some(19.0),
        "core" => Some(29.0),
        "plus" => Some(39.0),
        "prime" => Some(69.0),
        "pro" => Some(89.0),
        "max" => Some(149.0),
        "apex" => Some(189.0),
        _ => None,
    }
}

fn parse_iso_datetime(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn add_calendar_month_to_iso(raw_iso: &str) -> Option<String> {
    let parsed = parse_iso_datetime(raw_iso)?;
    Some(add_calendar_month(parsed).to_rfc3339())
}

fn add_days_to_iso(raw_iso: &str, days: i64) -> Option<String> {
    let parsed = parse_iso_datetime(raw_iso)?;
    Some((parsed + ChronoDuration::days(days)).to_rfc3339())
}

fn is_renew_allowed(status: &str, grace_until_iso: &str, now: DateTime<Utc>) -> bool {
    if status == SERVICE_STATUS_ACTIVE {
        return true;
    }
    if status != SERVICE_STATUS_GRACE_SUSPENDED {
        return false;
    }
    parse_iso_datetime(grace_until_iso)
        .map(|grace_until| now <= grace_until)
        .unwrap_or(false)
}

fn add_calendar_month(input: DateTime<Utc>) -> DateTime<Utc> {
    let year = input.year();
    let month = input.month();
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };

    let last_day = last_day_of_month(next_year, next_month);
    let day = input.day().min(last_day);
    let date = NaiveDate::from_ymd_opt(next_year, next_month, day)
        .expect("valid month/day during calendar month addition");
    let naive = date
        .and_hms_nano_opt(
            input.hour(),
            input.minute(),
            input.second(),
            input.nanosecond(),
        )
        .expect("valid time during calendar month addition");
    DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    for day in (28..=31).rev() {
        if NaiveDate::from_ymd_opt(year, month, day).is_some() {
            return day;
        }
    }
    28
}

fn backfill_hosting_service_defaults(item: &mut HostingServiceItem, fallback_panel_url: &str) {
    if item.status.trim().is_empty() {
        item.status = SERVICE_STATUS_ACTIVE.to_owned();
    }
    if item.expires_at.trim().is_empty() {
        item.expires_at = add_calendar_month_to_iso(&item.created_at)
            .unwrap_or_else(|| item.created_at.to_owned());
    }
    if item.grace_until.trim().is_empty() {
        item.grace_until =
            add_days_to_iso(&item.expires_at, 1).unwrap_or_else(|| item.expires_at.to_owned());
    }
    if item.billing_price_thb <= 0.0 {
        item.billing_price_thb = resolve_package_price(&item.package_name).unwrap_or(0.0);
    }
    if item
        .da_panel_url
        .as_deref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        item.da_panel_url = Some(fallback_panel_url.to_owned());
    }
}

fn backfill_hosting_service_list(
    items: &mut Vec<HostingServiceItem>,
    fallback_panel_url: &str,
) -> bool {
    let mut changed = false;
    for item in items.iter_mut() {
        let before = item.clone();
        backfill_hosting_service_defaults(item, fallback_panel_url);
        if before.status != item.status
            || before.expires_at != item.expires_at
            || before.grace_until != item.grace_until
            || before.suspended_at != item.suspended_at
            || (before.billing_price_thb - item.billing_price_thb).abs() > f64::EPSILON
            || before.notified_d1_at != item.notified_d1_at
            || before.notified_expired_at != item.notified_expired_at
            || before.notified_grace_end_at != item.notified_grace_end_at
            || before.da_panel_url != item.da_panel_url
        {
            changed = true;
        }
    }

    let mut by_domain: HashMap<String, HostingServiceItem> = HashMap::new();
    for item in items.iter().cloned() {
        let key = item.domain.to_ascii_lowercase();
        match by_domain.get(&key) {
            Some(existing) if existing.created_at >= item.created_at => {}
            _ => {
                by_domain.insert(key, item);
            }
        }
    }

    let mut normalized: Vec<HostingServiceItem> = by_domain.into_values().collect();
    normalized.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    if normalized.len() != items.len() {
        changed = true;
    }
    if changed {
        *items = normalized;
    }
    changed
}

fn extract_directadmin_error_message(body: &str) -> String {
    let from_pairs = url::form_urlencoded::parse(body.as_bytes()).find_map(|(key, value)| {
        if key == "text" || key == "details" || key == "error" {
            Some(value.into_owned())
        } else {
            None
        }
    });
    if let Some(value) = from_pairs {
        if !value.trim().is_empty() {
            return value;
        }
    }
    if let Some(detail) = body
        .split("text=")
        .nth(1)
        .and_then(|part| part.split('&').next())
    {
        if !detail.trim().is_empty() {
            return detail.to_owned();
        }
    }
    body.to_owned()
}

async fn push_notification(
    state: &AppState,
    username: &str,
    notification_type: &str,
    title: &str,
    message: &str,
) {
    let mut notifications = state.notifications_by_user.write().await;
    notifications
        .entry(username.to_owned())
        .or_default()
        .push(NotificationItem {
            id: Uuid::new_v4().to_string(),
            notification_type: notification_type.to_owned(),
            title: title.to_owned(),
            message: message.to_owned(),
            created_at: now_iso(),
            read: false,
        });
}

fn normalize_da_panel_url(raw_url: &str) -> String {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return "https://dcadmin.reverz.in.th/".to_owned();
    }

    let mut normalized = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_owned()
    } else {
        format!("https://{trimmed}")
    };

    if !normalized.ends_with('/') {
        normalized.push('/');
    }

    normalized
}

fn to_slip_storage_reference(ref_id: &str) -> String {
    format!("slip:{}", ref_id.trim())
}

fn normalize_slip_ref(ref_id: &str) -> String {
    ref_id.trim().to_ascii_lowercase()
}

fn normalize_receiver_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_uppercase()
}

fn normalize_receiver_id(receiver_id: &str) -> String {
    receiver_id.trim().to_ascii_lowercase()
}

fn parse_slip_amount(raw_amount: &serde_json::Value) -> Option<f64> {
    match raw_amount {
        serde_json::Value::Number(number) => number.as_f64(),
        serde_json::Value::String(value) => value.trim().parse::<f64>().ok(),
        _ => None,
    }
}

async fn fetch_bangkok_current_date(
    timeout_ms: u64,
) -> Result<String, (StatusCode, String, String)> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
        .map_err(|err| {
            eprintln!("[fetch_bangkok_current_date] failed to build client: {err}");
            (
                StatusCode::BAD_GATEWAY,
                "SLIP_API_ERROR".to_owned(),
                format!("Failed to initialize time API client: {err}"),
            )
        })?;

    let response = client
        .get(BANGKOK_TIME_API_URL)
        .header("cache-control", "no-store")
        .send()
        .await
        .map_err(|err| {
            eprintln!("[fetch_bangkok_current_date] request error: {err}");
            (
                StatusCode::BAD_GATEWAY,
                "SLIP_API_ERROR".to_owned(),
                format!("Failed to fetch Thai current date: {err}"),
            )
        })?;

    let status = response.status();
    let body = response.text().await.map_err(|err| {
        eprintln!("[fetch_bangkok_current_date] failed reading response body: {err}");
        (
            StatusCode::BAD_GATEWAY,
            "SLIP_API_ERROR".to_owned(),
            format!("Failed to read Thai time API response: {err}"),
        )
    })?;

    if !status.is_success() {
        let message = if body.trim().is_empty() {
            format!("Thai time API returned status {}", status.as_u16())
        } else {
            body
        };
        eprintln!(
            "[fetch_bangkok_current_date] non-success status={} body={}",
            status.as_u16(),
            message
        );
        return Err((
            StatusCode::BAD_GATEWAY,
            "SLIP_API_ERROR".to_owned(),
            message,
        ));
    }

    let payload = serde_json::from_str::<BangkokTimeApiResponse>(&body).map_err(|err| {
        eprintln!(
            "[fetch_bangkok_current_date] invalid payload parse_error={} body={}",
            err, body
        );
        (
            StatusCode::BAD_GATEWAY,
            "SLIP_API_INVALID_RESPONSE".to_owned(),
            format!("Thai time API response format is invalid: {err}"),
        )
    })?;

    let date = payload.date.trim().to_owned();
    if date.is_empty() {
        return Err((
            StatusCode::BAD_GATEWAY,
            "SLIP_API_INVALID_RESPONSE".to_owned(),
            "Thai time API date is missing".to_owned(),
        ));
    }

    Ok(date)
}

fn slip_date_matches_date_prefix(slip_date: &str, thai_date: &str) -> bool {
    slip_date.trim().starts_with(thai_date.trim())
}

fn is_valid_image_data_url(value: &str) -> bool {
    if value.len() > BANKING_SLIP_MAX_IMG_CHARS {
        return false;
    }

    let Some((header, payload)) = value.split_once(',') else {
        return false;
    };

    let header = header.to_ascii_lowercase();
    let is_allowed_mime = header.starts_with("data:image/jpeg;")
        || header.starts_with("data:image/jpg;")
        || header.starts_with("data:image/png;");
    if !is_allowed_mime || !header.contains(";base64") {
        return false;
    }

    !payload.is_empty()
        && payload
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '+' || ch == '/' || ch == '=')
}

fn compute_login_proof(username: &str, password: &str, nonce: &str) -> String {
    let raw = format!("{username}:{password}:{nonce}:{LOGIN_PEPPER}");
    let digest = Sha256::digest(raw.as_bytes());

    let mut output = String::with_capacity(digest.len() * 2);
    for value in digest {
        output.push_str(&format!("{value:02x}"));
    }
    output
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn require_env(name: &str) -> Result<String, String> {
    let value = std::env::var(name).map_err(|_| format!("missing required env: {name}"))?;
    if value.trim().is_empty() {
        return Err(format!("missing required env: {name}"));
    }
    Ok(value)
}

fn parse_env_u64(name: &str, fallback: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}

fn parse_env_usize(name: &str, fallback: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}

fn parse_env_string(name: &str, fallback: &str) -> String {
    std::env::var(name)
        .ok()
        .map(|raw| raw.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_owned())
}

fn validate_receiver_phone(phone: &str) -> Result<(), String> {
    if phone.len() != 10 || !phone.starts_with('0') || !phone.chars().all(|ch| ch.is_ascii_digit())
    {
        return Err("TRUEMONEY_RECEIVER_PHONE must be 10 digits and start with 0".to_owned());
    }

    Ok(())
}

async fn allow_rate_limit(state: &AppState, key: &str) -> bool {
    let now = unix_now_secs();
    let mut hits = state.rate_limit_hits.write().await;
    let entries = hits.entry(key.to_owned()).or_default();

    entries.retain(|ts| now.saturating_sub(*ts) <= 60);
    if entries.len() >= state.rate_limit_per_minute {
        return false;
    }

    entries.push(now);
    true
}

fn normalized_client_ip(headers: &HeaderMap) -> String {
    let forwarded = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let fallback = headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match forwarded.or(fallback) {
        Some(candidate) if candidate.parse::<std::net::IpAddr>().is_ok() => candidate.to_owned(),
        _ => "unknown".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use tower::ServiceExt;

    #[test]
    fn extract_voucher_hash_accepts_valid_url() {
        let hash = extract_voucher_hash("https://gift.truemoney.com/campaign/?v=ABC123xyz")
            .expect("valid URL should parse");
        assert_eq!(hash, "ABC123xyz");
    }

    #[test]
    fn extract_voucher_hash_rejects_multiple_v_params() {
        let err = extract_voucher_hash("https://gift.truemoney.com/campaign/?v=ABC123&v=DEF456")
            .unwrap_err();
        assert_eq!(err, "INVALID_VOUCHER_URL");
    }

    #[test]
    fn validate_image_data_url_accepts_supported_images() {
        assert!(is_valid_image_data_url("data:image/jpeg;base64,QUJDRA=="));
        assert!(is_valid_image_data_url("data:image/png;base64,AAECAwQ="));
        assert!(!is_valid_image_data_url("data:text/plain;base64,QUJDRA=="));
        assert!(!is_valid_image_data_url("data:image/jpeg;base64,"));
    }

    #[test]
    fn normalize_receiver_fields_should_be_strict() {
        assert_eq!(
            normalize_receiver_name("  Master   Mathakan   Tongeam  "),
            "MASTER MATHAKAN TONGEAM"
        );
        assert_eq!(normalize_receiver_id(" XXX-X-X8407-X "), "xxx-x-x8407-x");
    }

    #[test]
    fn slip_date_prefix_match_works() {
        assert!(slip_date_matches_date_prefix(
            "2026-03-07T11:29:00.000Z",
            "2026-03-07"
        ));
        assert!(!slip_date_matches_date_prefix(
            "2026-03-06T23:59:59.000Z",
            "2026-03-07"
        ));
    }

    #[test]
    fn parse_hosting_order_reference_works() {
        let parsed = parse_hosting_order_reference("order:Start:uffasas.com");
        assert_eq!(parsed, Some(("Start".to_owned(), "uffasas.com".to_owned())));

        assert!(parse_hosting_order_reference("order::uffasas.com").is_none());
        assert!(parse_hosting_order_reference("slip:123456").is_none());
    }

    #[test]
    fn add_calendar_month_handles_month_boundaries() {
        let march = DateTime::parse_from_rfc3339("2026-03-10T07:30:54+00:00")
            .expect("date should parse")
            .with_timezone(&Utc);
        let april = add_calendar_month(march);
        assert_eq!(april.to_rfc3339(), "2026-04-10T07:30:54+00:00");

        let jan_31 = DateTime::parse_from_rfc3339("2026-01-31T10:00:00+00:00")
            .expect("date should parse")
            .with_timezone(&Utc);
        let feb = add_calendar_month(jan_31);
        assert_eq!(feb.to_rfc3339(), "2026-02-28T10:00:00+00:00");
    }

    #[test]
    fn renew_window_validation_works() {
        let now = DateTime::parse_from_rfc3339("2026-03-10T07:30:54+00:00")
            .expect("date should parse")
            .with_timezone(&Utc);
        assert!(is_renew_allowed(
            SERVICE_STATUS_ACTIVE,
            "2026-03-11T07:30:54+00:00",
            now
        ));
        assert!(is_renew_allowed(
            SERVICE_STATUS_GRACE_SUSPENDED,
            "2026-03-10T08:30:54+00:00",
            now
        ));
        assert!(!is_renew_allowed(
            SERVICE_STATUS_GRACE_SUSPENDED,
            "2026-03-10T06:30:54+00:00",
            now
        ));
        assert!(!is_renew_allowed(
            SERVICE_STATUS_SUSPENDED_EXPIRED,
            "2026-03-11T07:30:54+00:00",
            now
        ));
    }

    #[test]
    fn collect_hosting_services_dedupes_domain_and_keeps_latest() {
        let items = collect_hosting_services(
            &[
                TopupTransaction {
                    tx_id: "1".to_owned(),
                    username: "root".to_owned(),
                    voucher_hash: "order:Start:Example.com".to_owned(),
                    amount_thb: -10.0,
                    status: "success".to_owned(),
                    error_code: None,
                    message: "Hosting order: Start (Example.com)".to_owned(),
                    created_at: "2026-03-10T07:00:00.000Z".to_owned(),
                },
                TopupTransaction {
                    tx_id: "2".to_owned(),
                    username: "root".to_owned(),
                    voucher_hash: "order:Plus:example.com".to_owned(),
                    amount_thb: -39.0,
                    status: "success".to_owned(),
                    error_code: None,
                    message: "Hosting order: Plus (example.com)".to_owned(),
                    created_at: "2026-03-10T08:00:00.000Z".to_owned(),
                },
                TopupTransaction {
                    tx_id: "3".to_owned(),
                    username: "root".to_owned(),
                    voucher_hash: "order:Lite:site-two.com".to_owned(),
                    amount_thb: -19.0,
                    status: "success".to_owned(),
                    error_code: None,
                    message: "Hosting order: Lite (site-two.com)".to_owned(),
                    created_at: "2026-03-10T09:00:00.000Z".to_owned(),
                },
                TopupTransaction {
                    tx_id: "4".to_owned(),
                    username: "root".to_owned(),
                    voucher_hash: "order:Lite:ignored.com".to_owned(),
                    amount_thb: -19.0,
                    status: "failed".to_owned(),
                    error_code: Some("FAILED".to_owned()),
                    message: "failed".to_owned(),
                    created_at: "2026-03-10T10:00:00.000Z".to_owned(),
                },
            ],
            "https://dcadmin.reverz.in.th/",
        );

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].domain, "site-two.com");
        assert_eq!(items[1].domain, "example.com");
        assert_eq!(items[1].package_name, "Plus");
    }

    #[tokio::test]
    async fn wallet_requires_authentication() {
        let app = build_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/topup/wallet")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn hosting_services_requires_authentication() {
        let app = build_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/hosting/services")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn notifications_requires_authentication() {
        let app = build_test_app().await;

        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/notifications")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(get_response.status(), StatusCode::UNAUTHORIZED);

        let post_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/notifications/mark-read")
                    .body(Body::from("{}"))
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(post_response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn admin_login_rejects_invalid_credentials() {
        let app = build_test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/admin/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"wrong@example.com","password":"bad","nonce":"1","proof":"x"}"#,
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn admin_login_accepts_valid_credentials_and_role() {
        let state = build_test_state().await;
        let app = build_app(state);

        let email = "pattaraphon16p@gmail.com";
        let password = "0833697042";
        let nonce = "1700000000000";
        let proof = compute_login_proof(email, password, nonce);
        let body = serde_json::json!({
            "email": email,
            "password": password,
            "nonce": nonce,
            "proof": proof,
        })
        .to_string();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/admin/login")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should decode");
        let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json should parse");
        assert_eq!(json["session"]["role"], "admin");
    }

    #[tokio::test]
    async fn admin_summary_requires_admin_role() {
        let state = build_test_state().await;
        let app = build_app(state.clone());

        let root_token = issue_access_token(
            "root",
            UserRole::Root,
            state.auth_config.as_ref(),
            unix_now_secs(),
        )
        .expect("token should issue");
        let root_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/admin/summary")
                    .header("authorization", format!("Bearer {root_token}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(root_response.status(), StatusCode::FORBIDDEN);

        let admin_token = issue_access_token(
            "pattaraphon16p@gmail.com",
            UserRole::Admin,
            state.auth_config.as_ref(),
            unix_now_secs(),
        )
        .expect("token should issue");
        let admin_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/admin/summary")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(admin_response.status(), StatusCode::OK);

        let blocked_user_endpoint = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/topup/wallet")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(blocked_user_endpoint.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn admin_recent_services_masks_da_password() {
        let state = build_test_state().await;
        {
            let mut hosting_services = state.hosting_services_by_user.write().await;
            hosting_services.insert(
                "root".to_owned(),
                vec![sample_service(
                    SERVICE_STATUS_ACTIVE,
                    "2099-03-10T10:00:00+00:00",
                    "2099-03-11T10:00:00+00:00",
                )],
            );
        }

        let app = build_app(state.clone());
        let admin_token = issue_access_token(
            "pattaraphon16p@gmail.com",
            UserRole::Admin,
            state.auth_config.as_ref(),
            unix_now_secs(),
        )
        .expect("token should issue");
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/admin/recent-services?limit=20")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should decode");
        let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json should parse");
        let masked = json["items"][0]["da_password_masked"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        assert_ne!(masked, "secret");
        assert!(masked.contains("******"));
    }

    #[tokio::test]
    async fn admin_user_wallets_requires_admin_role() {
        let state = build_test_state().await;
        let app = build_app(state.clone());

        let root_token =
            issue_access_token("root", UserRole::Root, state.auth_config.as_ref(), unix_now_secs())
                .expect("token should issue");
        let root_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/admin/user-wallets")
                    .header("authorization", format!("Bearer {root_token}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(root_response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn admin_user_wallets_returns_per_user_balances() {
        let state = build_test_state().await;
        {
            let mut wallets = state.wallets_by_user.write().await;
            wallets.insert(
                "alice".to_owned(),
                WalletAccount {
                    username: "alice".to_owned(),
                    balance_thb: 12.5,
                    updated_at: now_iso(),
                },
            );
            wallets.insert(
                "bob".to_owned(),
                WalletAccount {
                    username: "bob".to_owned(),
                    balance_thb: 34.0,
                    updated_at: now_iso(),
                },
            );
        }

        let app = build_app(state.clone());
        let admin_token = issue_access_token(
            "pattaraphon16p@gmail.com",
            UserRole::Admin,
            state.auth_config.as_ref(),
            unix_now_secs(),
        )
        .expect("token should issue");
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/admin/user-wallets")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should decode");
        let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json should parse");
        assert_eq!(json["ok"], true);
        let items = json["items"].as_array().cloned().unwrap_or_default();
        assert!(items.iter().any(|row| row["username"] == "alice" && row["balance_thb"] == 12.5));
        assert!(items.iter().any(|row| row["username"] == "bob" && row["balance_thb"] == 34.0));
    }

    #[tokio::test]
    async fn banking_slip_requires_authentication() {
        let app = build_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/topup/banking/slip/redeem")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"img":"data:image/jpeg;base64,QUJDRA=="}"#))
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn hosting_services_returns_unique_domains_for_user() {
        let state = build_test_state().await;
        {
            let mut transactions = state.transactions_by_user.write().await;
            transactions.insert(
                "root".to_owned(),
                vec![
                    TopupTransaction {
                        tx_id: "1".to_owned(),
                        username: "root".to_owned(),
                        voucher_hash: "order:Start:uffasas.com".to_owned(),
                        amount_thb: -10.0,
                        status: "success".to_owned(),
                        error_code: None,
                        message: "Hosting order: Start (uffasas.com)".to_owned(),
                        created_at: "2026-03-10T07:30:54.127960500+00:00".to_owned(),
                    },
                    TopupTransaction {
                        tx_id: "2".to_owned(),
                        username: "root".to_owned(),
                        voucher_hash: "order:Plus:UFFASAS.com".to_owned(),
                        amount_thb: -39.0,
                        status: "success".to_owned(),
                        error_code: None,
                        message: "Hosting order: Plus (UFFASAS.com)".to_owned(),
                        created_at: "2026-03-10T07:35:54.127960500+00:00".to_owned(),
                    },
                    TopupTransaction {
                        tx_id: "3".to_owned(),
                        username: "root".to_owned(),
                        voucher_hash: "order:Lite:site-two.com".to_owned(),
                        amount_thb: -19.0,
                        status: "success".to_owned(),
                        error_code: None,
                        message: "Hosting order: Lite (site-two.com)".to_owned(),
                        created_at: "2026-03-10T07:40:54.127960500+00:00".to_owned(),
                    },
                ],
            );
        }
        let app = build_app(state.clone());
        let access_token = issue_access_token(
            "root",
            UserRole::Root,
            state.auth_config.as_ref(),
            unix_now_secs(),
        )
        .expect("token should issue");

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/hosting/services")
                    .header("authorization", format!("Bearer {access_token}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should decode");
        let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json should parse");
        assert_eq!(json["ok"], true);
        assert_eq!(json["total_active"], 2);
    }

    #[tokio::test]
    async fn notifications_mark_read_flow_updates_unread_count() {
        let state = build_test_state().await;
        {
            let mut notifications = state.notifications_by_user.write().await;
            notifications.insert(
                "root".to_owned(),
                vec![
                    NotificationItem {
                        id: "n1".to_owned(),
                        notification_type: "service_created".to_owned(),
                        title: "Service created".to_owned(),
                        message: "A".to_owned(),
                        created_at: now_iso(),
                        read: false,
                    },
                    NotificationItem {
                        id: "n2".to_owned(),
                        notification_type: "service_renewed".to_owned(),
                        title: "Service renewed".to_owned(),
                        message: "B".to_owned(),
                        created_at: now_iso(),
                        read: false,
                    },
                ],
            );
        }

        let app = build_app(state.clone());
        let access_token = issue_access_token(
            "root",
            UserRole::Root,
            state.auth_config.as_ref(),
            unix_now_secs(),
        )
        .expect("token should issue");

        let get_before = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/notifications?limit=20")
                    .header("authorization", format!("Bearer {access_token}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(get_before.status(), StatusCode::OK);
        let bytes_before = to_bytes(get_before.into_body(), usize::MAX)
            .await
            .expect("body should decode");
        let before_json: serde_json::Value =
            serde_json::from_slice(&bytes_before).expect("json should parse");
        assert_eq!(before_json["unread_count"], 2);

        let mark = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/notifications/mark-read")
                    .header("authorization", format!("Bearer {access_token}"))
                    .body(Body::from("{}"))
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(mark.status(), StatusCode::OK);

        let get_after = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/notifications?limit=20")
                    .header("authorization", format!("Bearer {access_token}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        let bytes_after = to_bytes(get_after.into_body(), usize::MAX)
            .await
            .expect("body should decode");
        let after_json: serde_json::Value =
            serde_json::from_slice(&bytes_after).expect("json should parse");
        assert_eq!(after_json["unread_count"], 0);
    }

    #[tokio::test]
    async fn renew_service_requires_authentication() {
        let app = build_test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/hosting/services/renew")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"domain":"uffasas.com"}"#))
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn renew_service_returns_not_found_when_domain_missing() {
        let state = build_test_state().await;
        let app = build_app(state.clone());
        let access_token = issue_access_token(
            "root",
            UserRole::Root,
            state.auth_config.as_ref(),
            unix_now_secs(),
        )
        .expect("token should issue");

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/hosting/services/renew")
                    .header("authorization", format!("Bearer {access_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"domain":"uffasas.com"}"#))
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn renew_service_returns_bad_request_when_grace_window_expired() {
        let state = build_test_state().await;
        {
            let mut hosting_services = state.hosting_services_by_user.write().await;
            hosting_services.insert(
                "root".to_owned(),
                vec![sample_service(
                    SERVICE_STATUS_GRACE_SUSPENDED,
                    "2026-03-01T10:00:00+00:00",
                    "2026-03-02T10:00:00+00:00",
                )],
            );
        }
        let app = build_app(state.clone());
        let access_token = issue_access_token(
            "root",
            UserRole::Root,
            state.auth_config.as_ref(),
            unix_now_secs(),
        )
        .expect("token should issue");
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/hosting/services/renew")
                    .header("authorization", format!("Bearer {access_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"domain":"uffasas.com"}"#))
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn renew_service_returns_bad_request_when_balance_insufficient() {
        let state = build_test_state().await;
        {
            let mut hosting_services = state.hosting_services_by_user.write().await;
            hosting_services.insert(
                "root".to_owned(),
                vec![sample_service(
                    SERVICE_STATUS_ACTIVE,
                    "2099-03-10T10:00:00+00:00",
                    "2099-03-11T10:00:00+00:00",
                )],
            );
        }
        let app = build_app(state.clone());
        let access_token = issue_access_token(
            "root",
            UserRole::Root,
            state.auth_config.as_ref(),
            unix_now_secs(),
        )
        .expect("token should issue");
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/hosting/services/renew")
                    .header("authorization", format!("Bearer {access_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"domain":"uffasas.com"}"#))
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn renew_service_success_charges_wallet_and_updates_expiry() {
        let state = build_test_state().await;
        {
            let mut hosting_services = state.hosting_services_by_user.write().await;
            hosting_services.insert(
                "root".to_owned(),
                vec![sample_service(
                    SERVICE_STATUS_ACTIVE,
                    "2099-03-10T10:00:00+00:00",
                    "2099-03-11T10:00:00+00:00",
                )],
            );
        }
        {
            let mut wallets = state.wallets_by_user.write().await;
            wallets.insert(
                "root".to_owned(),
                WalletAccount {
                    username: "root".to_owned(),
                    balance_thb: 100.0,
                    updated_at: now_iso(),
                },
            );
        }

        let app = build_app(state.clone());
        let access_token = issue_access_token(
            "root",
            UserRole::Root,
            state.auth_config.as_ref(),
            unix_now_secs(),
        )
        .expect("token should issue");
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/hosting/services/renew")
                    .header("authorization", format!("Bearer {access_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"domain":"uffasas.com"}"#))
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(response.status(), StatusCode::OK);

        let wallets = state.wallets_by_user.read().await;
        let balance = wallets
            .get("root")
            .map(|wallet| wallet.balance_thb)
            .unwrap_or(0.0);
        assert_eq!(balance, 90.0);

        let services = state.hosting_services_by_user.read().await;
        let service = services
            .get("root")
            .and_then(|items| items.first())
            .expect("service should exist");
        assert_eq!(service.status, SERVICE_STATUS_ACTIVE);
        assert_eq!(service.expires_at, "2099-04-10T10:00:00+00:00");
    }

    #[tokio::test]
    async fn lifecycle_worker_d1_notification_is_not_duplicated() {
        let state = build_test_state().await;
        let near_expiry = (Utc::now() + ChronoDuration::hours(12)).to_rfc3339();
        let grace_until = (Utc::now() + ChronoDuration::hours(36)).to_rfc3339();

        {
            let mut hosting_services = state.hosting_services_by_user.write().await;
            hosting_services.insert(
                "root".to_owned(),
                vec![sample_service(
                    SERVICE_STATUS_ACTIVE,
                    &near_expiry,
                    &grace_until,
                )],
            );
        }

        run_lifecycle_tick(&state)
            .await
            .expect("tick should succeed");
        run_lifecycle_tick(&state)
            .await
            .expect("tick should succeed");

        let notifications = state.notifications_by_user.read().await;
        let count = notifications
            .get("root")
            .map(|items| {
                items
                    .iter()
                    .filter(|item| item.notification_type == "service_d1")
                    .count()
            })
            .unwrap_or(0);
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn wallet_accepts_valid_access_token() {
        let state = build_test_state().await;
        {
            let mut wallets = state.wallets_by_user.write().await;
            wallets.insert(
                "root".to_owned(),
                WalletAccount {
                    username: "root".to_owned(),
                    balance_thb: 15.0,
                    updated_at: now_iso(),
                },
            );
        }
        let app = build_app(state.clone());
        let access_token = issue_access_token(
            "root",
            UserRole::Root,
            state.auth_config.as_ref(),
            unix_now_secs(),
        )
        .expect("token should issue");

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/topup/wallet")
                    .header("authorization", format!("Bearer {access_token}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn redeem_blocks_replay_before_external_call() {
        let state = build_test_state().await;
        {
            let mut redeemed = state.redeemed_vouchers.write().await;
            redeemed.insert("abc123".to_owned());
        }
        let app = build_app(state.clone());
        let access_token = issue_access_token(
            "root",
            UserRole::Root,
            state.auth_config.as_ref(),
            unix_now_secs(),
        )
        .expect("token should issue");

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/topup/truemoney/redeem")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {access_token}"))
                    .body(Body::from(
                        r#"{"voucher_url":"https://gift.truemoney.com/campaign/?v=ABC123"}"#,
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::CONFLICT);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should decode");
        let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json should parse");
        assert_eq!(json["success"], false);
        assert_eq!(json["amount"], 0.0);
        assert_eq!(json["error_code"], "VOUCHER_ALREADY_REDEEMED");
    }

    #[tokio::test]
    async fn redeem_invalid_url_returns_truemoney_contract() {
        let state = build_test_state().await;
        let app = build_app(state.clone());
        let access_token = issue_access_token(
            "root",
            UserRole::Root,
            state.auth_config.as_ref(),
            unix_now_secs(),
        )
        .expect("token should issue");

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/topup/truemoney/redeem")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {access_token}"))
                    .body(Body::from(r#"{"voucher_url":"https://example.com/?v=x"}"#))
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should decode");
        let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json should parse");
        assert_eq!(json["success"], false);
        assert_eq!(json["amount"], 0.0);
        assert_eq!(json["error_code"], "INVALID_VOUCHER_URL");
    }

    #[tokio::test]
    async fn logout_revokes_refresh_family() {
        let state = build_test_state().await;
        let now = unix_now_secs();
        let refresh_token = new_refresh_token();
        let refresh_hash = hash_refresh_token(&refresh_token, &state.auth_config.refresh_secret);
        {
            let mut sessions = state.refresh_sessions.write().await;
            sessions.insert(
                refresh_hash,
                RefreshSession {
                    username: "root".to_owned(),
                    role: UserRole::Root.as_str().to_owned(),
                    family_id: "family-1".to_owned(),
                    issued_at_unix: now,
                    expires_at_unix: now + 600,
                    revoked_at_unix: None,
                },
            );
        }

        let access_token =
            issue_access_token("root", UserRole::Root, state.auth_config.as_ref(), now)
                .expect("token should issue");
        let app = build_app(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/logout")
                    .header("authorization", format!("Bearer {access_token}"))
                    .header("cookie", format!("refresh_token={refresh_token}"))
                    .body(Body::from("{}"))
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::OK);

        let sessions = state.refresh_sessions.read().await;
        let revoked = sessions
            .values()
            .all(|item| item.family_id != "family-1" || item.revoked_at_unix.is_some());
        assert!(revoked);
    }

    #[tokio::test]
    async fn refresh_endpoint_rotates_refresh_cookie() {
        let state = build_test_state().await;
        let now = unix_now_secs();
        let refresh_token = new_refresh_token();
        let refresh_hash = hash_refresh_token(&refresh_token, &state.auth_config.refresh_secret);
        {
            let mut sessions = state.refresh_sessions.write().await;
            sessions.insert(
                refresh_hash,
                RefreshSession {
                    username: "root".to_owned(),
                    role: UserRole::Root.as_str().to_owned(),
                    family_id: "family-refresh".to_owned(),
                    issued_at_unix: now,
                    expires_at_unix: now + 1_000,
                    revoked_at_unix: None,
                },
            );
        }

        let app = build_app(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/refresh")
                    .header("cookie", format!("refresh_token={refresh_token}"))
                    .body(Body::from("{}"))
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::OK);
        let has_cookie = response.headers().get("set-cookie").is_some();
        assert!(has_cookie);

        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should decode");
        let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json should parse");
        assert!(json["accessToken"].as_str().unwrap_or_default().len() > 10);
    }

    async fn build_test_app() -> Router {
        let state = build_test_state().await;
        build_app(state)
    }

    async fn build_test_state() -> AppState {
        let temp_dir = std::env::temp_dir().join(format!("reverz-prod-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).expect("temp dir should exist");

        AppState {
            sessions_by_user: Arc::new(RwLock::new(HashMap::new())),
            wallets_by_user: Arc::new(RwLock::new(HashMap::new())),
            transactions_by_user: Arc::new(RwLock::new(HashMap::new())),
            hosting_services_by_user: Arc::new(RwLock::new(HashMap::new())),
            notifications_by_user: Arc::new(RwLock::new(HashMap::new())),
            redeemed_vouchers: Arc::new(RwLock::new(HashSet::new())),
            used_slip_refs: Arc::new(RwLock::new(HashSet::new())),
            refresh_sessions: Arc::new(RwLock::new(HashMap::new())),
            rate_limit_hits: Arc::new(RwLock::new(HashMap::new())),
            sessions_storage_file: Arc::new(temp_dir.join("sessions.json")),
            wallets_storage_file: Arc::new(temp_dir.join("wallets.json")),
            transactions_storage_file: Arc::new(temp_dir.join("topup_transactions.json")),
            hosting_services_storage_file: Arc::new(temp_dir.join("hosting_services.json")),
            notifications_storage_file: Arc::new(temp_dir.join("notifications.json")),
            redeemed_storage_file: Arc::new(temp_dir.join("redeemed_vouchers.json")),
            used_slip_refs_storage_file: Arc::new(temp_dir.join("used_slip_refs.json")),
            refresh_sessions_storage_file: Arc::new(temp_dir.join("refresh_sessions.json")),
            persistence_lock: Arc::new(Mutex::new(())),
            topup_commit_lock: Arc::new(Mutex::new(())),
            receiver_phone: Arc::new("0931959423".to_owned()),
            banking_receiver_id: Arc::new(DEFAULT_BANKING_RECEIVER_ID.to_owned()),
            banking_receiver_name: Arc::new(DEFAULT_BANKING_RECEIVER_NAME.to_owned()),
            banking_slip_api_url: Arc::new(DEFAULT_BANKING_SLIP_API_URL.to_owned()),
            truemoney_timeout_ms: 10_000,
            banking_slip_timeout_ms: 10_000,
            rate_limit_per_minute: 5,
            auth_config: Arc::new(AuthConfig {
                jwt_secret: "test-jwt-secret-12345678901234567890".to_owned(),
                refresh_secret: "test-refresh-secret-1234567890123".to_owned(),
                access_ttl_seconds: 86_400,
                refresh_ttl_seconds: 2_592_000,
                secure_cookies: false,
            }),
            admin_username: Arc::new("root".to_owned()),
            admin_password: Arc::new("root".to_owned()),
            admin_login_email: Arc::new("pattaraphon16p@gmail.com".to_owned()),
            admin_login_password: Arc::new("0833697042".to_owned()),
            da_url: Arc::new("https://dcadmin.reverz.in.th".to_owned()),
            da_username: Arc::new("test-da-user".to_owned()),
            da_password: Arc::new("test-da-pass".to_owned()),
            da_server_ip: Arc::new("158.173.159.171".to_owned()),
        }
    }

    fn sample_service(status: &str, expires_at: &str, grace_until: &str) -> HostingServiceItem {
        HostingServiceItem {
            domain: "uffasas.com".to_owned(),
            package_name: "Start".to_owned(),
            created_at: "2099-03-10T10:00:00+00:00".to_owned(),
            status: status.to_owned(),
            expires_at: expires_at.to_owned(),
            grace_until: grace_until.to_owned(),
            suspended_at: None,
            billing_price_thb: 10.0,
            notified_d1_at: None,
            notified_expired_at: None,
            notified_grace_end_at: None,
            da_username: Some("uffa1234".to_owned()),
            da_password: Some("secret".to_owned()),
            da_panel_url: Some("https://dcadmin.reverz.in.th/".to_owned()),
        }
    }
}
