#[cfg(test)]
mod tests {
    use crate::socks5::config::{Socks5Auth, Socks5Config};
    use crate::socks5::handler::select_auth_method;
    use crate::socks5::protocol::*;

    #[test]
    fn select_no_auth() {
        let config = Socks5Config::default();
        assert_eq!(select_auth_method(&[AUTH_NO_AUTH], &config), AUTH_NO_AUTH);
        assert_eq!(
            select_auth_method(&[AUTH_USERNAME_PASSWORD], &config),
            AUTH_NO_ACCEPTABLE
        );
    }

    #[test]
    fn select_username_password_auth() {
        let config = Socks5Config {
            auth: Socks5Auth::UsernamePassword {
                username: "user".into(),
                password: "pass".into(),
            },
            upstream_proxy: None,
            throttle_rx: None,
        };
        assert_eq!(
            select_auth_method(&[AUTH_NO_AUTH, AUTH_USERNAME_PASSWORD], &config),
            AUTH_USERNAME_PASSWORD
        );
        assert_eq!(
            select_auth_method(&[AUTH_NO_AUTH], &config),
            AUTH_NO_ACCEPTABLE
        );
    }

    #[test]
    fn target_addr_display() {
        assert_eq!(
            TargetAddr::Ipv4([127, 0, 0, 1], 8080).to_address_string(),
            "127.0.0.1:8080"
        );
        assert_eq!(
            TargetAddr::Domain("example.com".into(), 443).to_address_string(),
            "example.com:443"
        );
        assert_eq!(
            TargetAddr::Ipv6([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1], 80)
                .to_address_string(),
            "[::1]:80"
        );
    }

    #[test]
    fn target_addr_from_socket_addr() {
        use std::net::SocketAddr;

        let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
        let target = TargetAddr::from_socket_addr(addr);
        assert_eq!(target.to_address_string(), "127.0.0.1:3000");

        let addr: SocketAddr = "[::1]:443".parse().unwrap();
        let target = TargetAddr::from_socket_addr(addr);
        assert_eq!(target.to_address_string(), "[::1]:443");
    }
}
