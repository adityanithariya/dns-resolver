"use client";

import type { Message } from "@/lib/types";
import { cn } from "@/lib/utils";

interface Segment {
  key: string;
  label: string;
  count: number;
  colorClass: string;
  anchor: string;
}

function scrollToAnchor(anchor: string) {
  document.getElementById(anchor)?.scrollIntoView({ behavior: "smooth", block: "start" });
}

/**
 * A byte-map style strip: one block per DNS message section, sized by how
 * many entries it holds relative to the whole message. It's the one loud
 * element on the page — everything else stays quiet.
 */
export default function PacketStrip({ message }: { message: Message }) {
  const segments: Segment[] = [
    { key: "header", label: "HEADER", count: 3, colorClass: "bg-muted-foreground", anchor: "section-header" },
    {
      key: "question",
      label: "QUESTION",
      count: message.questions.length || 1,
      colorClass: "bg-primary",
      anchor: "section-question",
    },
    {
      key: "answer",
      label: "ANSWER",
      count: message.answers.length,
      colorClass: "bg-accent",
      anchor: "section-answer",
    },
    {
      key: "authority",
      label: "AUTHORITY",
      count: message.authorities.length,
      colorClass: "bg-violet-400",
      anchor: "section-authority",
    },
    {
      key: "additional",
      label: "ADDITIONAL",
      count: message.additionals.length,
      colorClass: "bg-violet-400",
      anchor: "section-additional",
    },
  ];

  const total = segments.reduce((sum, s) => sum + Math.max(s.count, 1), 0);

  return (
    <div className="mb-7">
      <div className="flex h-10 overflow-hidden rounded-md border border-border">
        {segments.map((s) => {
          const width = `${(Math.max(s.count, 1) / total) * 100}%`;
          const empty = s.key !== "header" && s.key !== "question" && s.count === 0;
          return (
            <button
              key={s.key}
              type="button"
              onClick={() => scrollToAnchor(s.anchor)}
              title={`${s.label}: ${s.count}`}
              style={{ width }}
              className={cn(
                "relative border-r border-border last:border-r-0",
                empty ? "bg-card" : "bg-secondary",
                "hover:brightness-125 transition-[filter]"
              )}
            >
              <span
                className={cn("absolute bottom-0 left-0 h-[3px] w-full", empty ? "bg-border" : s.colorClass)}
              />
            </button>
          );
        })}
      </div>
      <div className="mt-1.5 flex justify-between text-[11px] tracking-wide text-muted-foreground">
        {segments.map((s) => (
          <span key={s.key}>
            {s.label} · {s.count}
          </span>
        ))}
      </div>
    </div>
  );
}
