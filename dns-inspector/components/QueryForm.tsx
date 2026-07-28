"use client";

import { useState } from "react";
import { QUERY_TYPES } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Card, CardContent } from "@/components/ui/card";

export type QueryStatus = "idle" | "encoding" | "sending" | "decoding" | "error";

interface Props {
  name: string;
  setName: (name: string) => void;
  qtype: number;
  setQtype: (qtype: number) => void;
  onSubmit: () => void;
  status: QueryStatus;
  errorMessage: string | null;
}

const STATUS_LABEL: Record<QueryStatus, string> = {
  idle: "ready",
  encoding: "encoding query…",
  sending: "sending…",
  decoding: "decoding response…",
  error: "failed",
};

export default function QueryForm({ qtype, setQtype, name, setName, onSubmit, status, errorMessage }: Props) {

  const busy = status === "encoding" || status === "sending" || status === "decoding";

  return (
    <Card>
      <CardContent className="pt-4">
        <form
          onSubmit={(e) => {
            e.preventDefault();
            if (!name.trim() || busy) return;
            onSubmit();
          }}
          className="flex flex-wrap items-end gap-3"
        >
          <div className="flex flex-1 min-w-[240px] flex-col gap-1.5">
            <Label htmlFor="dns-name">Name</Label>
            <Input
              id="dns-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="example.com"
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <Label htmlFor="dns-qtype">Type</Label>
            <Select value={String(qtype)} onValueChange={(v) => setQtype(Number(v))}>
              <SelectTrigger id="dns-qtype" className="w-28">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {QUERY_TYPES.map((t) => (
                  <SelectItem key={t.value} value={String(t.value)}>
                    {t.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <Button type="submit" disabled={busy} className="h-10">
            {busy ? STATUS_LABEL[status] : "Send query"}
          </Button>

          {status === "error" && errorMessage && (
            <p className="w-full text-sm text-destructive">{errorMessage}</p>
          )}
        </form>
      </CardContent>
    </Card>
  );
}
