interface BrandLogoProps {
  className?: string;
  alt?: string;
}

export function BrandLogo({
  className = "h-8 w-auto",
  alt = "AgentOS",
}: BrandLogoProps) {
  return (
    <div
      className={`agentos-brand-wordmark inline-flex items-center gap-2 ${className}`}
      role="img"
      aria-label={alt}
    >
      <span
        className="agentos-brand-wordmark__mark flex h-8 w-8 items-center justify-center rounded-md text-sm font-semibold"
        aria-hidden="true"
      >
        A
      </span>
      <span className="agentos-brand-wordmark__name text-sm font-semibold tracking-tight">
        AgentOS
      </span>
    </div>
  );
}
