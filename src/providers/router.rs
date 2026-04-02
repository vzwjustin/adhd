use std::collections::HashMap;
use std::sync::Arc;

use super::traits::*;
use crate::util::errors::{AnchorError, Result};

/// Routes agent roles to providers based on capabilities, health, and preferences.
pub struct ProviderRouter {
    providers: Vec<Arc<dyn Provider>>,
    role_preferences: HashMap<AgentRole, Vec<usize>>,
    health_cache: HashMap<usize, ProviderHealth>,
}

impl ProviderRouter {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            role_preferences: HashMap::new(),
            health_cache: HashMap::new(),
        }
    }

    /// Register a provider.
    pub fn add_provider(&mut self, provider: Arc<dyn Provider>) -> usize {
        let idx = self.providers.len();
        self.providers.push(provider);
        idx
    }

    /// Set preferred provider ordering for a role.
    pub fn set_role_preference(&mut self, role: AgentRole, provider_indices: Vec<usize>) {
        self.role_preferences.insert(role, provider_indices);
    }

    /// Get the best available provider for a role.
    /// Considers: role preferences > health > capability matching.
    pub fn route(&self, role: AgentRole) -> Result<Arc<dyn Provider>> {
        // Try role-specific preferences first
        if let Some(preferred) = self.role_preferences.get(&role) {
            for &idx in preferred {
                if let Some(provider) = self.providers.get(idx) {
                    if self.is_usable(idx) {
                        return Ok(Arc::clone(provider));
                    }
                }
            }
        }

        // Fallback: any healthy provider
        for (idx, provider) in self.providers.iter().enumerate() {
            if self.is_usable(idx) {
                return Ok(Arc::clone(provider));
            }
        }

        Err(AnchorError::Provider(
            "No available provider for this role".to_string(),
        ))
    }

    /// Check health of all providers and cache results.
    pub async fn refresh_health(&mut self) {
        for (idx, provider) in self.providers.iter().enumerate() {
            let health = provider.health_check().await;
            tracing::info!("Provider {} ({}): {}", idx, provider.name(), health);
            self.health_cache.insert(idx, health);
        }
    }

    /// Is a specific provider usable (healthy or degraded)?
    fn is_usable(&self, idx: usize) -> bool {
        self.health_cache
            .get(&idx)
            .map(|h| h.is_usable())
            .unwrap_or(true) // Assume usable if not yet checked
    }

    /// Get all registered providers with their health states.
    pub fn provider_status(&self) -> Vec<ProviderStatus> {
        self.providers
            .iter()
            .enumerate()
            .map(|(idx, p)| ProviderStatus {
                name: p.name().to_string(),
                health: self
                    .health_cache
                    .get(&idx)
                    .cloned()
                    .unwrap_or(ProviderHealth::Healthy),
                capabilities: p.capabilities().clone(),
            })
            .collect()
    }

    pub fn has_providers(&self) -> bool {
        !self.providers.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct ProviderStatus {
    pub name: String,
    pub health: ProviderHealth,
    pub capabilities: ProviderCapabilities,
}
