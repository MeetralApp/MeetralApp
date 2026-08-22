/**
* Tear down live audio paths (best-effort) then persist the ended meeting.
* Audio stop must not block ending — device-lost / path errors are common
* exactly when the user needs to leave the session.
*/
export async function endLiveMeetingSession(options: {
  meetingId: string;
  stopOutbound: () => Promise<unknown>;
  stopInbound: () => Promise<unknown>;
  endMeeting: (id: string) => Promise<unknown>;
}): Promise<void> {
  await Promise.allSettled([
    options.stopOutbound(),
    options.stopInbound(),
  ]);
  await options.endMeeting(options.meetingId);
}
