import {
  establishControlSession,
  physicalHostSequenceSeed,
  resolveAndroidSerial,
  resolveEspPort,
} from "./lab.js";
import { SerialLineTransport } from "./serial.js";
import { UsbControlClient } from "./usb.js";

const port = await resolveEspPort();
await resolveAndroidSerial();

const control = new UsbControlClient(
  new SerialLineTransport(port),
  undefined,
  undefined,
  physicalHostSequenceSeed(),
);
try {
  await establishControlSession(control);
  await control.command("getState");
} finally {
  await control.close();
}

process.stdout.write(
  `${JSON.stringify({
    espControlPort: port,
    espControlProtocol: 1,
    espResponsive: true,
    androidAuthorized: true,
  })}\n`,
);
