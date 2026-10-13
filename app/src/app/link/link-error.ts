import { ProtocolError } from '../protocol/turret-protocol';

export function describeLinkError(error: unknown): string | null {
  if (error instanceof ProtocolError) {
    return error.message;
  }
  if (!(error instanceof DOMException)) {
    return String(error);
  }
  if (error.name === 'NotFoundError') {
    return null;
  }
  if (/0x80\b/i.test(error.message)) {
    return 'The turret does not know this command. Update the app or the firmware.';
  }
  if (/0x81\b/i.test(error.message)) {
    return 'The turret refused the command values.';
  }
  switch (error.name) {
    case 'SecurityError':
      return 'Pairing failed. Check the passkey, or run unpair on the turret console and try again.';
    case 'NetworkError':
      return 'Connection to the turret lost.';
    default:
      return error.message;
  }
}
