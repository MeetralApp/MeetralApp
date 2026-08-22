import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";

import SettingsSection from "./SettingsSection";

const DEVELOPER_NAME = "iamnhaan";

export default function AboutFooter() {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion(null));
  }, []);

  return (
    <SettingsSection title="About">
      <div className="flex flex-col gap-1 text-sm text-muted-foreground">
        <p className="m-0 font-medium text-foreground">Meetral</p>
        {version ? <p className="m-0">Version {version}</p> : null}
        <p className="m-0">Developed by {DEVELOPER_NAME}</p>
      </div>
    </SettingsSection>
  );
}
