import { Slider as SliderPrimitive } from "@base-ui/react/slider";

import { cn } from "#lib/utils";

/**
 * A single-thumb slider over a small set of discrete positions.
 *
 * Differs from `Slider` in two ways that matter for a handful of named steps
 * rather than a continuous range. Every position is marked on the track, so
 * the scale can be read before touching it. And the whole band around the
 * track is the pointer target: pressing anywhere in it moves the thumb to the
 * nearest position, so the control does not demand a hit on the thin track
 * itself or a grab of the thumb.
 */
function StepSlider({
  className,
  value,
  stepCount,
  onValueChange,
  onValueCommitted,
  ...props
}: Omit<
  SliderPrimitive.Root.Props<number>,
  | "value"
  | "defaultValue"
  | "min"
  | "max"
  | "step"
  | "onValueChange"
  | "onValueCommitted"
  | "thumbAlignment"
> & {
  /**
   * Zero-based position of the thumb, or `null` when no position is in effect
   * yet: the track and its markers stay, but the thumb and fill are withheld
   * rather than pretending a position, and the first interaction chooses one.
   */
  value: number | null;
  /** How many positions the track offers; must be at least one. */
  stepCount: number;
  /** Fires with the position under the thumb while it moves. */
  onValueChange?: (value: number) => void;
  /** Fires with the position the thumb settles on when released. */
  onValueCommitted?: (value: number) => void;
}) {
  const last = Math.max(0, stepCount - 1);
  const unresolved = value === null;
  return (
    <SliderPrimitive.Root
      data-slot="step-slider"
      data-unresolved={unresolved ? "" : undefined}
      className={cn("w-full", className)}
      // The primitive needs a number to place its thumb; parking the withheld
      // thumb at the start keeps keyboard steps from the leading edge sane.
      value={value ?? 0}
      min={0}
      max={last}
      step={1}
      thumbAlignment="edge"
      onValueChange={(next) => onValueChange?.(next)}
      onValueCommitted={(next) => onValueCommitted?.(next)}
      {...props}
    >
      <SliderPrimitive.Control className="relative flex h-8 w-full cursor-pointer touch-none items-center select-none data-disabled:cursor-default data-disabled:opacity-50">
        <SliderPrimitive.Track
          data-slot="step-slider-track"
          className="relative h-3 w-full overflow-hidden rounded-full bg-muted select-none"
        >
          <SliderPrimitive.Indicator
            data-slot="step-slider-range"
            className={cn(
              "h-full bg-foreground/15 select-none",
              unresolved && "invisible",
            )}
          />
          {/* One marker per position. The thumb is edge-aligned, so its centre
              travels a span inset by half its width at either end; the markers
              live in that same inset box so each sits exactly where the thumb
              centre lands, and the thumb covers the current one. */}
          <span
            aria-hidden="true"
            className="pointer-events-none absolute inset-y-0 left-1.5 right-1.5"
          >
            {Array.from({ length: stepCount }, (_, index) => (
              <span
                key={index}
                className="absolute top-1/2 size-1 -translate-x-1/2 -translate-y-1/2 rounded-full bg-foreground/25"
                style={{ left: `${last === 0 ? 50 : (index / last) * 100}%` }}
              />
            ))}
          </span>
        </SliderPrimitive.Track>
        <SliderPrimitive.Thumb
          data-slot="step-slider-thumb"
          className={cn(
            "relative block h-5 w-3 shrink-0 rounded-full border border-border bg-background shadow-sm ring-ring/50 transition-[box-shadow] select-none focus-visible:ring-3 focus-visible:outline-hidden active:ring-3 disabled:pointer-events-none",
            unresolved && "invisible",
          )}
        />
      </SliderPrimitive.Control>
    </SliderPrimitive.Root>
  );
}

export { StepSlider };
