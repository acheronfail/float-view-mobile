import { SerialPort } from 'serialport';
import type { SerialPortOption } from '../contract.ts';

export async function listFixturePorts(): Promise<SerialPortOption[]> {
  const ports = await SerialPort.list();
  return ports
    .map((port) => ({
      path: port.path,
      label: [port.path, port.manufacturer, port.productId].filter(Boolean).join(' — '),
    }))
    .sort((left, right) => left.path.localeCompare(right.path));
}
