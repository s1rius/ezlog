<script setup lang="ts">
// This starter template is using Vue 3 <script setup> SFCs
// Check out https://vuejs.org/api/sfc-script-setup.html#script-setup
import { invoke } from "@tauri-apps/api/tauri";

import { type Event, listen } from '@tauri-apps/api/event'
import { onMounted, ref } from "vue";
import Modal from './Modal.vue'

const logs = ref<Record[]>([]);
const addRecords = (items: Record[]) => {
  logs.value = []
  logs.value.push(...items)
  showTable.value = logs.value.length > 0
}

const showModal = ref(false)
const showTable = ref(false)
const currentPath = ref("")
const currentExtra = ref("")

const logColors = new Map();

logColors.set('Trace', "rgb(156, 163, 175)");
logColors.set('Debug', "rgb(33, 150, 243)");
logColors.set('Info', "rgb(76, 175, 80)");
logColors.set('Warn', "rgb(255, 193, 7)");
logColors.set('Error', "rgb(244, 67, 54)");

type Header = {
  timestamp: 0,
  version: 2,
  cipher: string,
}

type HeaderWithExtra = {
  header: Header,
  extra: "",
  extra_encode: ""
}

export interface Record {
  n: string,
  l: string,
  g: string,
  t: string,
  d: number,
  m: string,
  c: string,
  f: string,
  y: number
}

// to avoid white screen
onMounted(async () => {
  setTimeout(() => {
    setupAppWindow()
  }, 200);
});

async function setupAppWindow() {
  const appWindow = (await import('@tauri-apps/api/window')).appWindow
  appWindow.show();
}

async function fetchLogs(path: string, k: string, n: string) {
  await invoke('parse_log_file_to_records', { filePath: path, key: k, nonce: n })
    .then((logs: any) => {
      let records: Record[] = JSON.parse(logs).map((item: string) => <Record>JSON.parse(item));
      addRecords(records)
    })
    .catch((error: any) => {
      console.error('Error fetching logs:', error);
    });
}

async function parse_header_and_extra(path: string) {
  console.log('parse file dropped:', path);
  currentPath.value = path;
  await invoke('parse_header_and_extra', { filePath: path }).then(async (result: any) => {
    const header_extra = JSON.parse(result as string) as HeaderWithExtra
    currentExtra.value = header_extra.extra_encode + ":\n" + header_extra.extra
    const noEncrypt = "NONE" == header_extra.header.cipher;
    if (noEncrypt) {
      fetchLogs(path, "", "")
    } else {
      showModal.value = true;
    }

  }).catch((error: any) => {
    console.error("error", error);
  })
}

async function submit_with_key_and_nonce(key: string, nonce: string) {
  showModal.value = false;
  fetchLogs(currentPath.value, key, nonce);
}

function getColorClass(data: string) {
  const value = logColors.get(data)
  return value !== undefined ? value : "";
}

listen('tauri://file-drop', (event: Event<string[]>) => {
  if (event.payload && event.payload.length > 0) {
    const firstFilePath = event.payload[0];
    console.log('file dropped:', firstFilePath);
    parse_header_and_extra(firstFilePath)
  }
})

</script>

<template>
  <div class="container bg-white dark:bg-stone-700/0 w-full max-w-full h-max-full p-3">
    <div v-if="!showTable"
      class="container w-full max-w-screen h-[calc(95dvh)] max-h-5/6 border-dashed border-2 border-slate-200">
      <div class="absolute top-12 self-center w-1/2">
        <img src="./assets/drag&drop.png" />
      </div>
    </div>

    <table v-show="showTable" class="table table-striped table-bordered border-separate border-spacing-x-3">
      <thead>
        <tr>
          <th class="text-left text-slate-900 dark:text-white">Time</th>
          <th class="text-left text-slate-900 dark:text-white">Target</th>
          <th class="text-left w-30 mx-3 text-slate-900 dark:text-white">Level</th>
          <th class="text-left text-slate-900 dark:text-white">Message</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(log, index) in logs" :key="index">
          <td class="w-30 min-w-30 h-fit mx-1 whitespace-nowrap align-top" :style="{ color: getColorClass(log.l) }">{{
      log.t }}</td>
          <td class="mx-1 align-top" :style="{ color: getColorClass(log.l) }">{{ log.g }}</td>
          <td class="w-30 mx-3 align-top text-left" :style="{ color: getColorClass(log.l) }">{{ log.l }}</td>
          <td class="text-wrap text-left break-all" :style="{ color: getColorClass(log.l) }">{{ log.c }}</td>
        </tr>
      </tbody>
    </table>

    <modal :show="showModal" @submit="submit_with_key_and_nonce">
      <template #header>
        <h2>Fill Key and Nonce</h2>
        <div>{{ currentExtra }}</div>
      </template>
    </modal>
  </div>
</template>
