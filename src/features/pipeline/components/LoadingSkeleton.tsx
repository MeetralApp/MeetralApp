import { Skeleton } from "@/shared/ui/skeleton";
import { pipelineToolbarClasses } from "@/features/pipeline/lib/pipelineColors";
import { cn } from "@/shared/lib/utils";

/**
* App boot / Live loading shell — mirrors AppShell header grid + dual columns.
*/
export default function LoadingSkeleton() {
  return (
    <div
      className="flex h-screen w-full flex-col bg-background p-3"
      aria-busy="true"
      aria-label="Loading application"
    >
      <header className="grid shrink-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2">
        <Skeleton className="size-9 rounded-md" />
        <div className="flex min-w-0 justify-center justify-self-stretch px-1">
          <Skeleton className="h-8 w-full max-w-[min(24rem,50vw)] rounded-md" />
        </div>
        <Skeleton className="size-9 justify-self-end rounded-md" />
      </header>

      <div className="mt-2 flex min-h-0 flex-1 overflow-hidden">
        <section
          className={cn(
            "flex min-w-0 flex-1 flex-col overflow-hidden",
            pipelineToolbarClasses.paneSurface,
          )}
        >
          <div
            className={cn(
              "flex min-h-8 shrink-0 items-center gap-2 px-3 py-1.5",
              pipelineToolbarClasses.paneToolbarRail,
            )}
          >
            <Skeleton className="h-7 w-28 rounded-md" />
            <div className="min-w-0 flex-1" />
            <Skeleton className="size-6 rounded-md" />
          </div>
          <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-hidden bg-card px-3 py-3">
            <Skeleton className="h-3 w-[70%] rounded-sm" />
            <Skeleton className="h-3 w-full rounded-sm" />
            <Skeleton className="h-3 w-[88%] rounded-sm" />
            <Skeleton className="h-3 w-[60%] rounded-sm" />
          </div>
        </section>

        <div
          className={cn("shrink-0", pipelineToolbarClasses.resizeHandle)}
          aria-hidden
        />

        <section
          className={cn(
            "flex min-w-0 flex-1 flex-col overflow-hidden",
            pipelineToolbarClasses.paneSurface,
          )}
        >
          <div
            className={cn(
              "flex min-h-8 shrink-0 items-center gap-2 px-3 py-1.5",
              pipelineToolbarClasses.paneToolbarRail,
            )}
          >
            <Skeleton className="h-7 w-28 rounded-md" />
            <div className="min-w-0 flex-1" />
            <Skeleton className="size-6 rounded-md" />
          </div>
          <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-hidden bg-card px-3 py-3">
            <Skeleton className="h-3 w-[65%] rounded-sm" />
            <Skeleton className="h-3 w-full rounded-sm" />
            <Skeleton className="h-3 w-[80%] rounded-sm" />
            <Skeleton className="h-3 w-[55%] rounded-sm" />
          </div>
        </section>
      </div>
    </div>
  );
}
