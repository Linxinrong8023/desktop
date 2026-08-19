import { useQuery } from "@tanstack/react-query";
import type { TaskDiffScope } from "@ora/contracts";
import { useContractsClient } from "../../contracts-client-context";
import { queryKeys } from "./query-keys";

/** Loads the current Git snapshot for one task-owned worktree. */
export function useTaskDiff(
  taskId: string,
  scope: TaskDiffScope,
  enabled = true,
) {
  const client = useContractsClient();
  return useQuery({
    queryKey: queryKeys.taskDiff(taskId, scope),
    queryFn: () => client.task.getDiff({ taskId, scope }),
    enabled: enabled && taskId !== "",
  });
}

/** Loads every discussion message associated with one task diff. */
export function useTaskDiffComments(taskId: string) {
  const client = useContractsClient();
  return useQuery({
    queryKey: queryKeys.taskDiffComments(taskId),
    queryFn: () =>
      client.task
        .listDiffComments({ taskId })
        .then((response) => response.comments),
  });
}
