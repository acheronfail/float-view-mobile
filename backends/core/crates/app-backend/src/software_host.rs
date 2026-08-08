use app_coordinator::ApplicationActor;
use domain_types::RuntimeGeneration;
use host_ports::{
    AdapterEvent, BluetoothAuthorization, BluetoothConnectionState, BluetoothNotification,
    BluetoothRadio, CompletionStatus, HostCommand, LocationAuthorization, OperationCompletion,
    PermissionKind, PlatformEvent,
};
use test_software_adapters::SoftwareVescPeripheral;

#[derive(Default)]
pub(crate) struct SoftwareHost {
    peripheral: SoftwareVescPeripheral,
    adapter_order: u64,
    connection_generation: u64,
}

impl SoftwareHost {
    pub(crate) fn pump(&mut self, application: &ApplicationActor) {
        let Ok(Some(command)) = application.try_host_command() else {
            return;
        };
        let generation = command.generation;
        let operation = command.operation;
        let accepted = self
            .handle(application, generation, command.command)
            .is_ok();
        let _ = application.complete(OperationCompletion {
            generation,
            operation,
            status: if accepted {
                CompletionStatus::Success
            } else {
                CompletionStatus::Failed
            },
            payload: Vec::new(),
        });
    }

    fn handle(
        &mut self,
        application: &ApplicationActor,
        generation: RuntimeGeneration,
        command: HostCommand,
    ) -> Result<(), app_coordinator::ActorError> {
        match command {
            HostCommand::BluetoothStartScan => {
                self.event(
                    application,
                    generation,
                    AdapterEvent::BluetoothState {
                        radio: BluetoothRadio::PoweredOn,
                        authorization: BluetoothAuthorization::Granted,
                    },
                )?;
                self.event(
                    application,
                    generation,
                    AdapterEvent::BluetoothDevice {
                        device_token: b"software-vesc".to_vec(),
                        name: Some("Floatwheel Software".into()),
                        rssi: -42,
                    },
                )
            }
            HostCommand::BluetoothConnect { .. } => {
                self.connection_generation = self.connection_generation.saturating_add(1);
                self.event(
                    application,
                    generation,
                    AdapterEvent::BluetoothConnection {
                        connection_generation: self.connection_generation,
                        state: BluetoothConnectionState::Connecting,
                    },
                )?;
                self.event(
                    application,
                    generation,
                    AdapterEvent::BluetoothConnection {
                        connection_generation: self.connection_generation,
                        state: BluetoothConnectionState::Ready,
                    },
                )
            }
            HostCommand::BluetoothDisconnect {
                connection_generation,
            } => {
                self.connection_generation = connection_generation;
                self.event(
                    application,
                    generation,
                    AdapterEvent::BluetoothConnection {
                        connection_generation,
                        state: BluetoothConnectionState::Disconnected,
                    },
                )
            }
            HostCommand::BluetoothWrite {
                connection_generation,
                bytes,
            } => {
                if let Some(bytes) = self.peripheral.response(&bytes) {
                    self.adapter_order = self.adapter_order.saturating_add(1);
                    application.submit_bluetooth(BluetoothNotification {
                        generation,
                        connection_generation,
                        adapter_order: self.adapter_order,
                        bytes,
                    })?;
                }
                Ok(())
            }
            HostCommand::PresentPermission {
                permission: PermissionKind::Bluetooth,
            } => self.event(
                application,
                generation,
                AdapterEvent::BluetoothState {
                    radio: BluetoothRadio::PoweredOn,
                    authorization: BluetoothAuthorization::Granted,
                },
            ),
            HostCommand::PresentPermission { .. } | HostCommand::LocationStart { .. } => self
                .event(
                    application,
                    generation,
                    AdapterEvent::LocationAuthorization(LocationAuthorization::Background),
                ),
            HostCommand::SetBackgroundLogging { enabled } => self.event(
                application,
                generation,
                AdapterEvent::LoggingService { active: enabled },
            ),
            HostCommand::BluetoothStopScan
            | HostCommand::LocationStop
            | HostCommand::ReadResource { .. }
            | HostCommand::StageDocument { .. }
            | HostCommand::ResolveStorageRoot { .. }
            | HostCommand::AvailableSpace { .. }
            | HostCommand::MonotonicClock
            | HostCommand::WallClock
            | HostCommand::SecureRandom { .. }
            | HostCommand::Diagnostic { .. }
            | HostCommand::PresentSettings
            | HostCommand::PresentTheme { .. }
            | HostCommand::PresentServiceState { .. }
            | HostCommand::PresentExport { .. }
            | HostCommand::RuntimeReady { .. } => Ok(()),
        }
    }

    fn event(
        &mut self,
        application: &ApplicationActor,
        generation: RuntimeGeneration,
        event: AdapterEvent,
    ) -> Result<(), app_coordinator::ActorError> {
        self.adapter_order = self.adapter_order.saturating_add(1);
        application.submit_platform_event(PlatformEvent {
            generation,
            connection_generation: self.connection_generation,
            adapter_order: self.adapter_order,
            event,
        })
    }
}
