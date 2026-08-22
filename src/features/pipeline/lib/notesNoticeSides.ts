/** "You", "Meeting", or "You & Meeting" for shared Notes notices. */
export function notesNoticeSides(
  outbound: boolean,
  inbound: boolean,
): string {
  if (outbound && inbound) return "You & Meeting";
  if (outbound) return "You";
  if (inbound) return "Meeting";
  return "";
}
