import type { ReactElement, SVGProps } from "react";
import * as Flags from "country-flag-icons/react/3x2";
import { resolveCountryCode } from "@/shared/lib/languageFlags";

interface Props {
  /** ISO 3166-1 alpha-2. Optional when `languageCode` is provided. */
  countryCode?: string;
  /** ISO language code — resolved via shared flag map when country is missing. */
  languageCode?: string;
  className?: string;
}

type FlagComponent = (props: SVGProps<SVGSVGElement>) => ReactElement;

export default function FlagIcon({
  countryCode,
  languageCode,
  className,
}: Props) {
  const resolved = resolveCountryCode(languageCode ?? "", countryCode);
  if (!resolved) {
    return (
      <span className={`flag-fallback ${className ?? ""}`} aria-hidden>
        {languageCode?.slice(0, 2).toUpperCase() || "?"}
      </span>
    );
  }
  const Flag = (Flags as Record<string, FlagComponent>)[resolved];
  if (!Flag) {
    return (
      <span className={`flag-fallback ${className ?? ""}`} aria-hidden>
        {resolved}
      </span>
    );
  }
  return <Flag className={className} aria-hidden />;
}
