use super::runtime::run_role_runtime_loop;
use tensor_vm::app::{
    RoleServiceConfig, RuntimeRole, ServiceRuntimeConfig, role_wallet_address, runtime_node_config,
};

pub(super) fn run_miner_service(
    config: RoleServiceConfig<'_>,
) -> std::result::Result<String, String> {
    RoleRunLoop::miner().run(config)
}

pub(super) fn run_validator_service(
    config: RoleServiceConfig<'_>,
) -> std::result::Result<String, String> {
    RoleRunLoop::validator().run(config)
}

pub(super) fn run_proposer_service(
    config: RoleServiceConfig<'_>,
) -> std::result::Result<String, String> {
    RoleRunLoop::proposer().run(config)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RoleRunLoopKind {
    Miner,
    Validator,
    Proposer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RoleRunLoop {
    kind: RoleRunLoopKind,
}

impl RoleRunLoop {
    pub(super) fn miner() -> Self {
        Self {
            kind: RoleRunLoopKind::Miner,
        }
    }

    pub(super) fn validator() -> Self {
        Self {
            kind: RoleRunLoopKind::Validator,
        }
    }

    pub(super) fn proposer() -> Self {
        Self {
            kind: RoleRunLoopKind::Proposer,
        }
    }

    fn runtime_command(self) -> &'static str {
        match self.kind {
            RoleRunLoopKind::Miner => "miner_run",
            RoleRunLoopKind::Validator => "validator_run",
            RoleRunLoopKind::Proposer => "proposer_run",
        }
    }

    fn runtime_role(self) -> RuntimeRole {
        match self.kind {
            RoleRunLoopKind::Miner => RuntimeRole::Miner,
            RoleRunLoopKind::Validator => RuntimeRole::Validator,
            RoleRunLoopKind::Proposer => RuntimeRole::Proposer,
        }
    }

    pub(super) fn service_runtime_config(
        self,
        config: RoleServiceConfig<'_>,
    ) -> std::result::Result<ServiceRuntimeConfig, String> {
        let role = self.runtime_role();
        Ok(ServiceRuntimeConfig {
            runtime_command: self.runtime_command(),
            role,
            role_wallet_address: Some(role_wallet_address(config.wallet)?),
            node: runtime_node_config(
                config.data_dir,
                role,
                config.listen,
                config.p2p_listen,
                config.identity_seed,
                config.auth_token,
                config.max_requests,
            )?,
        })
    }

    fn run(self, config: RoleServiceConfig<'_>) -> std::result::Result<String, String> {
        let service_report = run_role_runtime_loop(self.service_runtime_config(config)?)?;
        Ok(self.format_report(config, &service_report))
    }

    pub(super) fn format_report(
        self,
        config: RoleServiceConfig<'_>,
        service_report: &str,
    ) -> String {
        match self.kind {
            RoleRunLoopKind::Miner => format!(
                "command=miner_run\nrole=miner\nwallet={}\ndevice={}\nnode={}\nrole_runtime_ready=true\n{service_report}",
                config.wallet,
                config.device.unwrap_or("unknown"),
                config.node
            ),
            RoleRunLoopKind::Validator => format!(
                "command=validator_run\nrole=validator\nwallet={}\nnode={}\nreference_verifier_ready=true\nrole_runtime_ready=true\n{service_report}",
                config.wallet, config.node
            ),
            RoleRunLoopKind::Proposer => format!(
                "command=proposer_run\nrole=proposer\nwallet={}\nnode={}\nproposer_ready=true\nrole_runtime_ready=true\n{service_report}",
                config.wallet, config.node
            ),
        }
    }
}
