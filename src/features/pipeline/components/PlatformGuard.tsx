import { useEffect, useState } from "react";

import LoadingSkeleton from "@/features/pipeline/components/LoadingSkeleton";
import { getPlatform } from "@/shared/lib/api/platformApi";
import { Alert, AlertDescription, AlertTitle } from "@/shared/ui/alert";

const SUPPORTED_PLATFORMS = new Set(["windows", "macos"]);

export default function PlatformGuard({ children }: { children: React.ReactNode }) {
  const [platform, setPlatform] = useState<string | null>(null);

  useEffect(() => {
    getPlatform()
      .then(setPlatform)
      .catch(() => setPlatform("unknown"));
  }, []);

  if (platform === null) {
    return <LoadingSkeleton />;
  }

  if (!SUPPORTED_PLATFORMS.has(platform)) {
    return (
      <div className="flex min-h-screen items-center justify-center p-6">
        <Alert variant="destructive" className="max-w-lg" role="alert">
          <AlertTitle>Unsupported platform</AlertTitle>
          <AlertDescription className="space-y-2">
            <p className="m-0">
              Meetral supports Windows 10/11 and macOS 13+ (Apple Silicon).
              Install a virtual audio router (VoiceMeeter on Windows, BlackHole
              on macOS) for Teams integration.
            </p>
            <p className="m-0">Detected platform: {platform}</p>
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  return <>{children}</>;
}
