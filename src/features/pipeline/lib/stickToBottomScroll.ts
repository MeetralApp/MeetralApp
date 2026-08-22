export const STICK_TO_BOTTOM_THRESHOLD_PX = 80;

export function distanceFromBottom(element: HTMLElement): number {
  return element.scrollHeight - element.scrollTop - element.clientHeight;
}

export function isNearBottom(
  element: HTMLElement,
  threshold = STICK_TO_BOTTOM_THRESHOLD_PX,
): boolean {
  return distanceFromBottom(element) <= threshold;
}

export function scrollElementToBottom(element: HTMLElement): void {
  element.scrollTop = element.scrollHeight;
}
