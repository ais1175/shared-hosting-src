use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub device: String,
    pub ip: String,
    pub location: String,
    pub last_active: String,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionView {
    pub id: String,
    pub device: String,
    pub ip: String,
    pub location: String,
    pub last_active: String,
    pub is_current: bool,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub ok: bool,
    pub message: String,
    pub error_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterSessionRequest {
    pub session_id: String,
    pub device: String,
    pub ip: String,
    pub location: String,
    pub last_active: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub proof: String,
    pub nonce: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminLoginRequest {
    pub email: String,
    pub password: String,
    pub proof: String,
    pub nonce: String,
}

#[derive(Debug, Serialize)]
pub struct AuthSessionPayload {
    pub username: String,
    pub role: String,
    #[serde(rename = "loggedInAt")]
    pub logged_in_at: String,
    #[serde(rename = "accessToken")]
    pub access_token: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub ok: bool,
    pub session: AuthSessionPayload,
}

#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub ok: bool,
    #[serde(rename = "accessToken")]
    pub access_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletAccount {
    pub username: String,
    pub balance_thb: f64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopupTransaction {
    pub tx_id: String,
    pub username: String,
    pub voucher_hash: String,
    pub amount_thb: f64,
    pub status: String,
    pub error_code: Option<String>,
    pub message: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct WalletResponse {
    pub ok: bool,
    pub username: String,
    pub balance_thb: f64,
    pub receiver_phone: String,
    pub banking_receiver_id: String,
    pub banking_receiver_name: String,
}

#[derive(Debug, Serialize)]
pub struct RedeemResponse {
    pub success: bool,
    pub amount: f64,
    pub message: String,
    pub error_code: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TransactionsResponse {
    pub ok: bool,
    pub items: Vec<TopupTransactionView>,
}

#[derive(Debug, Serialize)]
pub struct TopupTransactionView {
    pub tx_id: String,
    pub voucher_hash: String,
    pub amount_thb: f64,
    pub status: String,
    pub error_code: Option<String>,
    pub message: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct RedeemRequest {
    pub voucher_url: String,
}

#[derive(Debug, Deserialize)]
pub struct BankingSlipRedeemRequest {
    pub img: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateHostingOrderRequest {
    pub domain: String,
    pub email: String,
    pub package_name: String,
    pub price: f64,
}

#[derive(Debug, Serialize)]
pub struct CreateHostingOrderResponse {
    pub ok: bool,
    pub message: String,
    pub da_username: Option<String>,
    pub da_password: Option<String>,
    pub da_panel_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostingServiceItem {
    pub domain: String,
    pub package_name: String,
    pub created_at: String,
    #[serde(default = "default_hosting_service_status")]
    pub status: String,
    #[serde(default)]
    pub expires_at: String,
    #[serde(default)]
    pub grace_until: String,
    #[serde(default)]
    pub suspended_at: Option<String>,
    #[serde(default)]
    pub billing_price_thb: f64,
    #[serde(default)]
    pub notified_d1_at: Option<String>,
    #[serde(default)]
    pub notified_expired_at: Option<String>,
    #[serde(default)]
    pub notified_grace_end_at: Option<String>,
    pub da_username: Option<String>,
    pub da_password: Option<String>,
    pub da_panel_url: Option<String>,
}

fn default_hosting_service_status() -> String {
    "active".to_owned()
}

#[derive(Debug, Serialize)]
pub struct HostingServicesResponse {
    pub ok: bool,
    pub total_active: usize,
    pub items: Vec<HostingServiceItem>,
}

#[derive(Debug, Deserialize)]
pub struct RenewHostingServiceRequest {
    pub domain: String,
}

#[derive(Debug, Serialize)]
pub struct RenewHostingServiceResponse {
    pub ok: bool,
    pub message: String,
    pub domain: String,
    pub status: String,
    pub expires_at: String,
    pub grace_until: String,
    pub charged_amount: f64,
    pub balance_thb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationItem {
    pub id: String,
    pub notification_type: String,
    pub title: String,
    pub message: String,
    pub created_at: String,
    pub read: bool,
}

#[derive(Debug, Serialize)]
pub struct NotificationsResponse {
    pub ok: bool,
    pub items: Vec<NotificationItem>,
    pub unread_count: usize,
}

#[derive(Debug, Serialize)]
pub struct MarkReadNotificationsResponse {
    pub ok: bool,
    pub unread_count: usize,
}

#[derive(Debug, Serialize)]
pub struct AdminSummaryResponse {
    pub ok: bool,
    pub total_users: usize,
    pub total_active_services: usize,
    pub total_services_all_status: usize,
    pub total_transactions: usize,
    pub wallet_total_thb: f64,
    pub unread_notifications_total: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminServiceView {
    pub owner_username: String,
    pub domain: String,
    pub package_name: String,
    pub status: String,
    pub created_at: String,
    pub expires_at: String,
    pub grace_until: String,
    pub da_username_masked: String,
    pub da_password_masked: String,
}

#[derive(Debug, Serialize)]
pub struct AdminServicesResponse {
    pub ok: bool,
    pub items: Vec<AdminServiceView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminTransactionView {
    pub tx_id: String,
    pub owner_username: String,
    pub voucher_hash_masked: String,
    pub voucher_method: String,
    pub amount_thb: f64,
    pub status: String,
    pub message: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct AdminTransactionsResponse {
    pub ok: bool,
    pub items: Vec<AdminTransactionView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminUserWalletView {
    pub username: String,
    pub balance_thb: f64,
}

#[derive(Debug, Serialize)]
pub struct AdminUserWalletsResponse {
    pub ok: bool,
    pub items: Vec<AdminUserWalletView>,
}

#[derive(Debug, Deserialize)]
pub struct TransactionsQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshSession {
    pub username: String,
    #[serde(default = "default_refresh_session_role")]
    pub role: String,
    pub family_id: String,
    pub issued_at_unix: u64,
    pub expires_at_unix: u64,
    pub revoked_at_unix: Option<u64>,
}

fn default_refresh_session_role() -> String {
    "root".to_owned()
}

#[derive(Debug, Clone)]
pub struct RedeemOutcome {
    pub success: bool,
    pub amount_thb: f64,
    pub error_code: Option<String>,
    pub message: String,
}
