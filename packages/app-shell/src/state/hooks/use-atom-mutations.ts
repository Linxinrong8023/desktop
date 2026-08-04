import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { Agent, Skill } from "@ora/contracts";
import { usePlatform } from "@ora/platform";
import { useContractsClient } from "../../contracts-client-context";
import { queryKeys } from "./query-keys";

/** Creates a configurable agent and refreshes the agent list. */
export function useCreateAgent() {
  const client = useContractsClient();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ name, description }: { name: string; description: string }) =>
      client.agent.create({ name, description }).then((response) => response.agent),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.agents });
    },
  });
}

/** Replaces a configurable agent's fields and refreshes the agent list. */
export function useUpdateAgent() {
  const client = useContractsClient();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ agent, name, description }: { agent: Agent; name: string; description: string }) =>
      client.agent.update({ agentId: agent.id, name, description }).then((response) => response.agent),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.agents });
    },
  });
}

/** Soft-deletes a configurable agent and refreshes the agent list. */
export function useDeleteAgent() {
  const client = useContractsClient();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ agentId }: { agentId: string }) =>
      client.agent.delete({ agentId }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.agents });
    },
  });
}

/** Creates a skill and refreshes the skill list. */
export function useCreateSkill() {
  const client = useContractsClient();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ name, description }: { name: string; description: string }) =>
      client.skill.create({ name, description }).then((response) => response.skill),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.skills });
    },
  });
}
/** Imports one host-selected skill folder and refreshes the skill list after success. */
export function useImportSkill() {
  const platform = usePlatform();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => {
      if (platform.skillFolderImport.kind === "unsupported") {
        return Promise.resolve(null);
      }
      return platform.skillFolderImport.importFolder();
    },
    onSuccess: (skill) => {
      if (skill !== null) {
        queryClient.invalidateQueries({ queryKey: queryKeys.skills });
      }
    },
  });
}

/** Replaces a skill's fields and refreshes the skill list. */
export function useUpdateSkill() {
  const client = useContractsClient();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ skill, name, description }: { skill: Skill; name: string; description: string }) =>
      client.skill.update({ skillId: skill.id, name, description }).then((response) => response.skill),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.skills });
    },
  });
}

/** Soft-deletes a skill and refreshes the skill list. */
export function useDeleteSkill() {
  const client = useContractsClient();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ skillId }: { skillId: string }) =>
      client.skill.delete({ skillId }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.skills });
    },
  });
}
