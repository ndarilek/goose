import { useState, type ButtonHTMLAttributes, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
  Copy,
  FolderOpen,
  MessageSquarePlus,
  MoreVertical,
  Pencil,
  Save,
  Trash2,
} from "lucide-react";
import { MessageResponse } from "@/shared/ui/ai-elements/message";
import {
  Avatar as AvatarRoot,
  AvatarFallback,
  AvatarImage,
} from "@/shared/ui/avatar";
import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { DetailField } from "@/shared/ui/detail-field";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";
import { PageColumns } from "@/shared/ui/page-columns";
import { DetailPageShell, PageHeader } from "@/shared/ui/page-shell";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/shared/ui/tooltip";
import { useAvatarSrc } from "@/shared/hooks/useAvatarSrc";
import type { Persona } from "@/shared/types/agents";
import {
  getPersonaInitials,
  getPersonaSource,
  isPersonaReadOnly,
} from "@/features/agents/lib/personaPresentation";

interface AgentDetailPageProps {
  persona: Persona;
  onBack: () => void;
  onEdit: (persona: Persona) => void;
  onReveal: (persona: Persona) => void;
  onStartChat?: (persona: Persona) => void;
  onCopyFile: (persona: Persona) => void;
  onSaveCopy: (persona: Persona) => void;
  onDuplicate: (persona: Persona) => void;
  onDelete: (persona: Persona) => void;
}

interface AgentHeaderActionButtonProps
  extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  icon: ReactNode;
}

function AgentHeaderActionButton({
  label,
  icon,
  type = "button",
  ...props
}: AgentHeaderActionButtonProps) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          type={type}
          size="icon-xs"
          variant="outline-flat"
          aria-label={label}
          {...props}
        >
          {icon}
          <span className="sr-only">{label}</span>
        </Button>
      </TooltipTrigger>
      <TooltipContent side="top" align="center" sideOffset={8}>
        <p>{label}</p>
      </TooltipContent>
    </Tooltip>
  );
}

function formatDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat(undefined, {
    month: "long",
    day: "numeric",
    year: "numeric",
  }).format(date);
}

export function AgentDetailPage({
  persona,
  onBack,
  onEdit,
  onReveal,
  onStartChat,
  onCopyFile,
  onSaveCopy,
  onDuplicate,
  onDelete,
}: AgentDetailPageProps) {
  const { t } = useTranslation(["agents", "common"]);
  const [menuOpen, setMenuOpen] = useState(false);
  const avatarSrc = useAvatarSrc(persona.avatar);
  const initials = getPersonaInitials(persona.displayName);
  const personaSource = getPersonaSource(persona);
  const canEditPersona = !isPersonaReadOnly(persona);
  const canDeletePersona = personaSource !== "builtin";
  const hasFileActions =
    personaSource === "file" && Boolean(persona.sourcePath);
  const sourceLabel =
    personaSource === "builtin"
      ? t("common:labels.builtIn")
      : personaSource === "file"
        ? t("card.fileBacked")
        : t("card.custom");
  const providerLabel = persona.provider || t("common:labels.none");
  const modelLabel = persona.model || t("common:labels.none");
  const moreLabel = t("view.more");

  return (
    <DetailPageShell>
      <div className="space-y-5 border-b border-border pb-6">
        <Button
          type="button"
          variant="back"
          size="sm"
          className="w-fit"
          onClick={onBack}
        >
          {t("view.backToAgents")}
        </Button>

        <PageHeader
          variant="detail"
          title={
            <span className="inline-flex min-w-0 items-center gap-3">
              <AvatarRoot className="size-12 shrink-0 border border-border-soft bg-muted/30">
                <AvatarImage
                  src={avatarSrc ?? undefined}
                  alt={persona.displayName}
                />
                <AvatarFallback className="text-base font-semibold">
                  {initials}
                </AvatarFallback>
              </AvatarRoot>
              <span className="min-w-0 truncate">{persona.displayName}</span>
            </span>
          }
          description={persona.systemPrompt}
          descriptionClassName="line-clamp-2 max-w-3xl leading-relaxed"
          actionsPlacement="below"
          actions={
            <>
              {onStartChat ? (
                <AgentHeaderActionButton
                  label={t("view.startChatShort")}
                  icon={<MessageSquarePlus className="size-3.5" />}
                  onClick={() => onStartChat(persona)}
                />
              ) : null}
              {canEditPersona ? (
                <AgentHeaderActionButton
                  label={t("common:actions.edit")}
                  icon={<Pencil className="size-3.5" />}
                  onClick={() => onEdit(persona)}
                />
              ) : null}
              {persona.sourcePath ? (
                <AgentHeaderActionButton
                  label={t("view.reveal")}
                  icon={<FolderOpen className="size-3.5" />}
                  onClick={() => onReveal(persona)}
                />
              ) : null}
              <DropdownMenu open={menuOpen} onOpenChange={setMenuOpen}>
                <DropdownMenuTrigger asChild>
                  <Button
                    type="button"
                    size="icon-xs"
                    variant="outline-flat"
                    aria-label={moreLabel}
                  >
                    <MoreVertical className="size-3.5" />
                    <span className="sr-only">{moreLabel}</span>
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" sideOffset={8}>
                  {hasFileActions ? (
                    <>
                      <DropdownMenuItem onSelect={() => onCopyFile(persona)}>
                        <Copy className="size-3.5" />
                        {t("view.copyFile")}
                      </DropdownMenuItem>
                      <DropdownMenuItem onSelect={() => onSaveCopy(persona)}>
                        <Save className="size-3.5" />
                        {t("view.saveCopy")}
                      </DropdownMenuItem>
                    </>
                  ) : null}
                  <DropdownMenuItem onSelect={() => onDuplicate(persona)}>
                    <Copy className="size-3.5" />
                    {t("editor.duplicate")}
                  </DropdownMenuItem>
                  {canDeletePersona ? (
                    <DropdownMenuItem
                      variant="destructive"
                      onSelect={() => onDelete(persona)}
                    >
                      <Trash2 className="size-3.5" />
                      {t("common:actions.delete")}
                    </DropdownMenuItem>
                  ) : null}
                </DropdownMenuContent>
              </DropdownMenu>
            </>
          }
          actionsClassName="gap-2"
        />
      </div>

      <PageColumns
        defaultSidebarSize={30}
        minSidebarSize={24}
        maxSidebarSize={38}
        minContentSize={52}
        sidebar={
          <aside className="space-y-5">
            <section className="space-y-5 border-b border-border pb-5">
              <DetailField label={t("view.source")}>
                <Badge variant="secondary">{sourceLabel}</Badge>
              </DetailField>

              {persona.sourcePath ? (
                <DetailField
                  label={t("view.filePath")}
                  contentAs="p"
                  contentClassName="break-all text-foreground"
                >
                  {persona.sourcePath}
                </DetailField>
              ) : null}

              <DetailField
                label={t("editor.provider")}
                contentAs="p"
                contentClassName="break-words"
              >
                {providerLabel}
              </DetailField>

              <DetailField
                label={t("editor.model")}
                contentAs="p"
                contentClassName="break-words"
              >
                {modelLabel}
              </DetailField>
            </section>

            <section className="space-y-5">
              <DetailField label={t("view.created")} contentAs="p">
                {formatDate(persona.createdAt)}
              </DetailField>
              <DetailField label={t("view.updated")} contentAs="p">
                {formatDate(persona.updatedAt)}
              </DetailField>
            </section>
          </aside>
        }
      >
        <section className="space-y-4 pb-6">
          <DetailField
            label={t("editor.systemPrompt")}
            meta={
              <span className="text-[10px] text-muted-foreground">
                {t("common:labels.characterCount", {
                  count: persona.systemPrompt.length,
                })}
              </span>
            }
          />
          <MessageResponse className="min-w-0 text-sm leading-6">
            {persona.systemPrompt || " "}
          </MessageResponse>
        </section>
      </PageColumns>
    </DetailPageShell>
  );
}
