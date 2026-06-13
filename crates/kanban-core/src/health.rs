#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_healthy_status_is_healthy() {
        assert!(HealthStatus::Healthy.is_healthy());
        assert!(!HealthStatus::Healthy.is_degraded());
    }

    #[test]
    fn test_degraded_status_is_degraded() {
        let s = HealthStatus::Degraded("disk full".into());
        assert!(s.is_degraded());
        assert!(!s.is_healthy());
    }

    #[test]
    fn test_unhealthy_is_neither_healthy_nor_degraded() {
        let s = HealthStatus::Unhealthy("connection lost".into());
        assert!(!s.is_healthy());
        assert!(!s.is_degraded());
    }

    #[test]
    fn test_health_status_display_healthy() {
        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
    }

    #[test]
    fn test_health_status_display_degraded() {
        assert_eq!(
            HealthStatus::Degraded("slow".into()).to_string(),
            "degraded: slow"
        );
    }

    #[test]
    fn test_health_status_display_unhealthy() {
        assert_eq!(
            HealthStatus::Unhealthy("gone".into()).to_string(),
            "unhealthy: gone"
        );
    }

    #[test]
    fn test_health_checker_trait_is_object_safe() {
        struct AlwaysHealthy;
        impl HealthChecker for AlwaysHealthy {
            fn check(&self) -> HealthStatus {
                HealthStatus::Healthy
            }
        }
        let _: &dyn HealthChecker = &AlwaysHealthy;
    }
}
