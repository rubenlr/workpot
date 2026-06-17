import { resyncDetailIfOpen } from "$lib/tray/logic/detail/detailRepoSync";
import type { RepoDto } from "$lib/types";

export function createTrayDetail() {
  let detailRepo = $state<RepoDto | null>(null);
  let focusTagOnDetailOpen = $state(false);

  function openDetail(repo: RepoDto) {
    detailRepo = repo;
  }

  function closeDetail() {
    detailRepo = null;
  }

  function openDetailWithTagFocus(repo: RepoDto) {
    detailRepo = repo;
    focusTagOnDetailOpen = true;
  }

  function clearTagFocusRequest() {
    focusTagOnDetailOpen = false;
  }

  function resync(repos: RepoDto[]) {
    detailRepo = resyncDetailIfOpen(repos, detailRepo);
  }

  return {
    get detailRepo() {
      return detailRepo;
    },
    get focusTagOnDetailOpen() {
      return focusTagOnDetailOpen;
    },
    openDetail,
    closeDetail,
    openDetailWithTagFocus,
    clearTagFocusRequest,
    resync,
  };
}

export type TrayDetail = ReturnType<typeof createTrayDetail>;
