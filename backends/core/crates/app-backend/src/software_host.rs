use app_coordinator::ApplicationActor;
use domain_types::{PlatformAssociation, RuntimeGeneration};
use host_ports::{
    AdapterEvent, BluetoothAuthorization, BluetoothConnectionState, BluetoothNotification,
    BluetoothRadio, CompletionStatus, HostCommand, LocationAuthorization, OperationCompletion,
    PermissionKind, PlatformCommand, PlatformEvent,
};
use std::sync::mpsc::SyncSender;
use test_software_adapters::SoftwareVescPeripheral;

#[derive(Default)]
pub(crate) struct SoftwareHost {
    peripheral: SoftwareVescPeripheral,
    adapter_order: u64,
    connection_generation: u64,
}

impl SoftwareHost {
    pub(crate) fn initialize(
        &mut self,
        application: &ApplicationActor,
    ) -> Result<(), app_coordinator::ActorError> {
        self.event(
            application,
            application.generation(),
            AdapterEvent::BluetoothState {
                radio: BluetoothRadio::PoweredOn,
                authorization: BluetoothAuthorization::Granted,
            },
        )
    }

    pub(crate) fn pump(
        &mut self,
        application: &ApplicationActor,
        platform_commands: Option<&SyncSender<PlatformCommand>>,
    ) {
        let Ok(Some(command)) = application.try_host_command() else {
            return;
        };
        if !Self::owns(&command.command)
            && let Some(platform_commands) = platform_commands
        {
            let _ = platform_commands.try_send(command);
            return;
        }
        let generation = command.generation;
        let operation = command.operation;
        if let HostCommand::BluetoothWrite {
            connection_generation,
            bytes,
        } = &command.command
        {
            let response = self.peripheral.response(bytes);
            let _ = application.complete(OperationCompletion {
                generation,
                operation,
                status: CompletionStatus::Success,
                payload: Vec::new(),
            });
            if let Some(bytes) = response {
                self.adapter_order = self.adapter_order.saturating_add(1);
                let _ = application.submit_bluetooth(BluetoothNotification {
                    generation,
                    connection_generation: *connection_generation,
                    adapter_order: self.adapter_order,
                    bytes,
                });
            }
            return;
        }
        let accepted = self
            .handle(application, generation, &command.command)
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

    fn owns(command: &HostCommand) -> bool {
        matches!(
            command,
            HostCommand::BluetoothStartScan
                | HostCommand::BluetoothStopScan
                | HostCommand::BluetoothConnect { .. }
                | HostCommand::BluetoothDisconnect { .. }
                | HostCommand::BluetoothWrite { .. }
                | HostCommand::PresentPermission {
                    permission: PermissionKind::Bluetooth
                }
        )
    }

    fn handle(
        &mut self,
        application: &ApplicationActor,
        generation: RuntimeGeneration,
        command: &HostCommand,
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
                        association: PlatformAssociation::new(b"software-vesc".to_vec())
                            .expect("fixed software association"),
                        name: Some("Floatwheel Software".into()),
                        rssi: -42,
                    },
                )
            }
            HostCommand::BluetoothConnect {
                connection_generation,
                ..
            } => {
                self.connection_generation = *connection_generation;
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
                self.connection_generation = *connection_generation;
                self.event(
                    application,
                    generation,
                    AdapterEvent::BluetoothConnection {
                        connection_generation: *connection_generation,
                        state: BluetoothConnectionState::Disconnected,
                    },
                )
            }
            HostCommand::BluetoothWrite { .. } => unreachable!("writes complete before responses"),
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
            HostCommand::BluetoothStopScan
            | HostCommand::SetBackgroundLogging { .. }
            | HostCommand::LocationStop
            | HostCommand::PresentSettings
            | HostCommand::RuntimeReady { .. } => Ok(()),
            HostCommand::PresentImport { .. } | HostCommand::PresentExport { .. } => {
                Err(app_coordinator::ActorError::InvalidInput)
            }
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
