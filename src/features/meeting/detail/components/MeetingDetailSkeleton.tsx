import { Skeleton } from "@/shared/ui/skeleton";
import { pipelineToolbarClasses } from "@/features/pipeline/lib/pipelineColors";
import { cn } from "@/shared/lib/utils";

/**
* Meeting Detail loading shell — mirrors Summary | Transcript split,
* headerless panes, floating search chip (top-right), summary generate chip, player.
*/
export default function MeetingDetailSkeleton() {
  return (
    <div
      className="flex h-full min-h-0 flex-col"
      aria-busy="true"
      aria-label="Loading meeting"
    >
      <div className="flex min-h-0 flex-1 overflow-hidden">
        {/* Summary ~38% */}
        <section
          className={cn(
            "relative flex w-[38%] min-w-0 flex-col overflow-hidden",
            pipelineToolbarClasses.paneSurface,
          )}
        >
          <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-hidden bg-card px-4 py-3 pb-14">
            <Skeleton className="ml-auto size-6 rounded-md" />
            <Skeleton className="h-3 w-20 rounded-sm" />
            <Skeleton className="h-3 w-full rounded-sm" />
            <Skeleton className="h-3 w-[92%] rounded-sm" />
            <Skeleton className="h-3 w-[78%] rounded-sm" />
            <Skeleton className="mt-2 h-3 w-24 rounded-sm" />
            <Skeleton className="h-3 w-full rounded-sm" />
            <Skeleton className="h-3 w-[88%] rounded-sm" />
          </div>
          <div className="pointer-events-none absolute right-3 bottom-3 flex items-end gap-2">
            <Skeleton className="size-10 rounded-full shadow-sm" />
            <Skeleton className="h-10 w-16 rounded-full shadow-sm" />
          </div>
        </section>

        <div
          className={cn("shrink-0", pipelineToolbarClasses.resizeHandle)}
          aria-hidden
        />

        {/* Transcript ~62% */}
        <section
          className={cn(
            "relative flex min-w-0 flex-1 flex-col overflow-hidden",
            pipelineToolbarClasses.paneSurface,
          )}
        >
          <div className="relative flex min-h-0 flex-1 flex-col gap-4 overflow-hidden bg-card px-3 py-3 pb-20">
            <Skeleton className="absolute top-2 right-4 size-8 rounded-md" />
            <div className="mt-10 flex flex-col gap-1.5">
              <Skeleton className="h-3 w-24 rounded-sm" />
              <Skeleton className="h-3 w-full rounded-sm" />
              <Skeleton className="h-3 w-[90%] rounded-sm" />
            </div>
            <div className="flex flex-col gap-1.5">
              <Skeleton className="h-3 w-20 rounded-sm" />
              <Skeleton className="h-3 w-full rounded-sm" />
              <Skeleton className="h-3 w-[82%] rounded-sm" />
            </div>
            <div className="flex flex-col gap-1.5">
              <Skeleton className="h-3 w-28 rounded-sm" />
              <Skeleton className="h-3 w-[95%] rounded-sm" />
            </div>
          </div>
          <div className="pointer-events-none absolute inset-x-0 bottom-0 z-10 p-2.5 pt-0">
            <Skeleton className="h-12 w-full rounded-lg" />
          </div>
        </section>
      </div>
    </div>
  );
}
