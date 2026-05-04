import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Copy, MoreVertical, Pencil, Save, Trash2 } from "lucide-react";
import { cn } from "@/shared/lib/cn";
import { Avatar, AvatarImage, AvatarFallback } from "@/shared/ui/avatar";
import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";
import { useAvatarSrc } from "@/shared/hooks/useAvatarSrc";
import type { Persona } from "@/shared/types/agents";
import {
  getPersonaInitials,
  getPersonaSource,
  isPersonaReadOnly,
} from "@/features/agents/lib/personaPresentation";

interface PersonaCardProps {
  persona: Persona;
  onSelect?: (persona: Persona) => void;
  onEdit?: (persona: Persona) => void;
  onDuplicate?: (persona: Persona) => void;
  onDelete?: (persona: Persona) => void;
  onCopyFile?: (persona: Persona) => void;
  onSaveCopy?: (persona: Persona) => void;
  isActive?: boolean;
}

export function PersonaCard({
  persona,
  onSelect,
  onEdit,
  onDuplicate,
  onDelete,
  onCopyFile,
  onSaveCopy,
  isActive = false,
}: PersonaCardProps) {
  const { t } = useTranslation(["agents", "common"]);
  const [menuOpen, setMenuOpen] = useState(false);

  const initials = getPersonaInitials(persona.displayName);
  const avatarSrc = useAvatarSrc(persona.avatar);
  const personaSource = getPersonaSource(persona);
  const canEditPersona = !isPersonaReadOnly(persona);
  const canDeletePersona = personaSource !== "builtin";
  const hasFileActions =
    personaSource === "file" && Boolean(persona.sourcePath);
  const isFeatured = personaSource === "builtin";
  const providerModelLabel = [persona.provider, persona.model]
    .filter(Boolean)
    .join(" / ");

  const handleCardKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.target !== event.currentTarget || menuOpen) {
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onSelect?.(persona);
    }
  };

  return (
    // biome-ignore lint/a11y/useSemanticElements: card contains nested menu buttons, so a native button is not valid here
    <div
      aria-label={t("card.ariaLabel", { name: persona.displayName })}
      role="button"
      onClick={() => !menuOpen && onSelect?.(persona)}
      onKeyDown={handleCardKeyDown}
      tabIndex={0}
      className={cn(
        "group relative flex cursor-pointer flex-col rounded-2xl border border-border-soft bg-background p-5",
        "transition-colors duration-200 motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-2",
        "hover:border-border hover:bg-muted/10 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
        isActive && "border-border bg-muted/20",
      )}
    >
      {isFeatured ? (
        <Badge
          variant="featured"
          className="absolute right-5 top-5 z-10 text-[10px]"
        >
          {t("card.featured")}
        </Badge>
      ) : null}

      <div className="flex items-start justify-between gap-3">
        <Avatar className="size-12 border border-border-soft bg-muted/30">
          <AvatarImage src={avatarSrc ?? undefined} alt={persona.displayName} />
          <AvatarFallback className="text-sm font-semibold">
            {initials}
          </AvatarFallback>
        </Avatar>

        <div className="relative z-20 -mr-2 -mt-2">
          <DropdownMenu open={menuOpen} onOpenChange={setMenuOpen}>
            <DropdownMenuTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="icon-xs"
                aria-label={t("card.options")}
                onClick={(e) => e.stopPropagation()}
                onKeyDown={(event) => event.stopPropagation()}
                className={cn(
                  "size-6 rounded-md text-muted-foreground hover:text-foreground",
                  menuOpen
                    ? "opacity-100"
                    : "opacity-0 group-hover:opacity-100 group-focus-within:opacity-100",
                )}
              >
                <MoreVertical className="size-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" sideOffset={4}>
              {hasFileActions && (
                <>
                  <DropdownMenuItem onSelect={() => onCopyFile?.(persona)}>
                    <Copy className="size-3.5" />
                    {t("view.copyFile")}
                  </DropdownMenuItem>
                  <DropdownMenuItem onSelect={() => onSaveCopy?.(persona)}>
                    <Save className="size-3.5" />
                    {t("view.saveCopy")}
                  </DropdownMenuItem>
                </>
              )}
              {canEditPersona && (
                <DropdownMenuItem onSelect={() => onEdit?.(persona)}>
                  <Pencil className="size-3.5" />
                  {t("common:actions.edit")}
                </DropdownMenuItem>
              )}
              <DropdownMenuItem onSelect={() => onDuplicate?.(persona)}>
                <Copy className="size-3.5" />
                {t("common:actions.duplicate")}
              </DropdownMenuItem>
              {canDeletePersona && (
                <DropdownMenuItem
                  variant="destructive"
                  onSelect={() => onDelete?.(persona)}
                >
                  <Trash2 className="size-3.5" />
                  {t("common:actions.delete")}
                </DropdownMenuItem>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      <div className="mt-4 min-w-0 space-y-1">
        <div className="flex min-w-0 items-center gap-2">
          <h3 className="min-w-0 flex-1 truncate text-sm font-medium leading-5 text-foreground">
            {persona.displayName}
          </h3>
        </div>

        {providerModelLabel ? (
          <p className="truncate text-[11px] leading-4 text-muted-foreground">
            {providerModelLabel}
          </p>
        ) : null}
      </div>

      <div aria-hidden="true" className="h-7 shrink-0" />

      <p className="line-clamp-3 max-w-2xl text-xs font-light leading-5 text-muted-foreground">
        {persona.systemPrompt}
      </p>
    </div>
  );
}
