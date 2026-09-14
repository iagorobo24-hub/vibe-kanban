import { AgentOSWordmark } from "@vibe/web-core/agentos-wordmark";

interface BrandLogoProps {
  className?: string;
  alt?: string;
}

export function BrandLogo({
  className = "h-8 w-auto",
  alt = "AgentOS",
}: BrandLogoProps) {
  return <AgentOSWordmark className={className} alt={alt} />;
}
