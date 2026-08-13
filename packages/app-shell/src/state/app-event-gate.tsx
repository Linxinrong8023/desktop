import { useEffect, useState, type ReactNode } from "react";
import type { ContractsClient } from "@ora/contracts";
import type { AppWindowOwnershipCapability, AppWindowOwnershipLease } from "@ora/platform";
import { useTranslation } from "react-i18next";
import { useAppEvents } from "./hooks/use-app-events";

type OwnershipState =
  | { type: "acquiring" }
  | { type: "waiting" }
  | { type: "owned" }
  | { type: "unavailable" };

interface AppEventGateProps {
  client: ContractsClient;
  ownership: AppWindowOwnershipCapability;
  children: ReactNode;
}

/** Prevents normal application work until this page exclusively owns the shell. */
export function AppEventGate({ client, ownership, children }: AppEventGateProps) {
  const [ownershipState, setOwnershipState] = useState<OwnershipState>({ type: "acquiring" });
  const { t } = useTranslation();

  useEffect(() => {
    const controller = new AbortController();
    let lease: AppWindowOwnershipLease | undefined;
    void ownership.acquire({
      signal: controller.signal,
      onWaiting: () => setOwnershipState({ type: "waiting" }),
    }).then((acquiredLease) => {
      if (controller.signal.aborted) {
        acquiredLease.release();
        return;
      }
      lease = acquiredLease;
      setOwnershipState({ type: "owned" });
    }).catch(() => {
      if (!controller.signal.aborted) setOwnershipState({ type: "unavailable" });
    });

    return () => {
      controller.abort();
      lease?.release();
    };
  }, [ownership]);

  if (ownershipState.type === "waiting") {
    return (
      <main className="flex min-h-dvh items-center justify-center bg-background px-6 text-foreground">
        <section className="w-full max-w-md space-y-4 rounded-xl border border-border bg-card p-6 shadow-sm">
          <h1 className="text-xl font-semibold">{t("appEvents.multipleClients.title")}</h1>
          <p className="text-sm text-muted-foreground">{t("appEvents.multipleClients.description")}</p>
        </section>
      </main>
    );
  }

  if (ownershipState.type === "unavailable") {
    return (
      <main className="flex min-h-dvh items-center justify-center bg-background px-6 text-foreground">
        <section className="w-full max-w-md space-y-4 rounded-xl border border-border bg-card p-6 shadow-sm">
          <h1 className="text-xl font-semibold">{t("appEvents.ownershipUnavailable.title")}</h1>
          <p className="text-sm text-muted-foreground">{t("appEvents.ownershipUnavailable.description")}</p>
        </section>
      </main>
    );
  }

  if (ownershipState.type !== "owned") {
    return <Connecting />;
  }

  return <AppEventStreamGate client={client}>{children}</AppEventStreamGate>;
}

/** Waits for the application event stream after this page owns the shell. */
function AppEventStreamGate({ client, children }: { client: ContractsClient; children: ReactNode }) {
  const { ready } = useAppEvents(client);
  if (!ready) return <Connecting />;
  return <>{children}</>;
}

/** Renders the shared startup state used while ownership or events are connecting. */
function Connecting() {
  const { t } = useTranslation();
  return (
    <main className="flex min-h-dvh items-center justify-center bg-background text-sm text-muted-foreground">
      {t("appEvents.connecting")}
    </main>
  );
}
