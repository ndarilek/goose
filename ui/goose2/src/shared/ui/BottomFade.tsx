export function BottomFade() {
  return (
    <div
      className="pointer-events-none sticky bottom-0 left-0 h-64 w-full"
      style={{
        background:
          "linear-gradient(to bottom, rgba(222,222,222,0) 0%, var(--canvas) 100%)",
        backdropFilter: "blur(3px)",
        WebkitBackdropFilter: "blur(3px)",
        maskImage:
          "linear-gradient(to bottom, transparent 0%, rgba(0,0,0,0.4) 50%, black 100%)",
        WebkitMaskImage:
          "linear-gradient(to bottom, transparent 0%, rgba(0,0,0,0.4) 50%, black 100%)",
      }}
      aria-hidden="true"
    />
  );
}
