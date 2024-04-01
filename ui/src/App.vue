<script setup lang="ts">
// This starter template is using Vue 3 <script setup> SFCs
// Check out https://vuejs.org/api/sfc-script-setup.html#script-setup
import { invoke } from "@tauri-apps/api/tauri";

import { type Event, listen } from '@tauri-apps/api/event'
import { ref } from "vue";
import Modal from './Modal.vue'

const logs = ref<Record[]>([]);
const add = (items: Record[]) => {
  logs.value.push(...items)
}

const showModal = ref(false)
const currentPath = ref("")
const currentExtra = ref("")

const logColors = new Map();

logColors.set('a', 1);
logColors.set('b', 2);
logColors.set('c', 3);

type Header = {
  timestamp: 0,
  version: 2,
  encrypt: 0,
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

async function fetchLogs(path: string, k: string, n: string) {
  await invoke('parse_log_file_to_records', { filePath: path, key: k, nonce: n })
    .then((logs: any) => {
      let records: Record[] = JSON.parse(logs).map((item: string) => <Record>JSON.parse(item));
      add(records)
    })
    .catch((error: any) => {
      console.error('Error fetching logs:', error);
    });
}

async function parse_header_and_extra(path: string) {
  console.log('parse file dropped:', path);
  currentPath.value = path;
  await invoke('parse_header_and_extra', { filePath: path }).then(async (result: any) => {
    const header = JSON.parse(result as string) as Header
    console.log(header)
    currentExtra.value = header.extra_encode + ":\n" + header.extra
    if (header.encrypt == 0) {
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

listen('tauri://file-drop', (event: Event<string[]>) => {
  if (event.payload && event.payload.length > 0) {
    const firstFilePath = event.payload[0];
    console.log('First file dropped:', firstFilePath);
    // Now you can do something with the first file path
    // For example, reading the file content or processing the file
    parse_header_and_extra(firstFilePath)
  }
})

</script>

<template>
  <div class="container bg-white dark:bg-stone-700/0 w-full max-w-full p-3">
    <table class="table table-striped table-bordered border-separate border-spacing-x-3">
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
          <td class="text-slate-900 dark:text-gray-400 w-30 min-w-30 h-fit mx-1 whitespace-nowrap align-top">{{ log.t }}</td>
          <td class="text-slate-900 dark:text-gray-400 mx-1 align-top">{{ log.g }}</td>
          <td class="text-slate-900 dark:text-gray-400 w-30 mx-3 align-top">{{ log.l }}</td>
          <td class="text-slate-900 dark:text-gray-400 text-wrap text-left break-all">{{ log.c }}</td>
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

<style scoped>
.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}
</style>
