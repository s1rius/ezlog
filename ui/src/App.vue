<script setup lang="ts">
// This starter template is using Vue 3 <script setup> SFCs
// Check out https://vuejs.org/api/sfc-script-setup.html#script-setup
import { invoke } from "@tauri-apps/api/tauri";
import Greet from "./components/Greet.vue";
import LogList from "./components/LogList.vue";

import { listen } from '@tauri-apps/api/event'

type Header = {
  timestamp: 0,
  version: 2,
  encrypt: 0,
  extra: "",
  extra_encode: ""
}

var record = {

}

async function fetchLogs(path: string) {
            await invoke('parse_log_file_to_records', {file_path: path})
                .then((logs: any) => {
                    ologs = logs;
                })
                .catch((error: any) => {
                    console.error('Error fetching logs:', error);
                });
        }

async function parse_header_and_extra(path: string) {
  console.log('parse file dropped:', path);
    await invoke('parse_header_and_extra', {filePath: path}).then((result: any) => {
        console.error("sdfasdf", result);
        const header = JSON.parse(result as string) as Header
        if (header.encrypt == 0) {

        } else {
          
        }
        console.log(header)
    }).catch((error: any) => {
      console.error("error", error);
    })
}

listen('tauri://file-drop', (event) => {
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
  <div class="container mx-auto p-8">
    <h1>Welcome to Tauri!</h1>

    <div class=" bg-white justify-center rounded-lg border border-gray-100 ">
      <a href="https://vitejs.dev" target="_blank">
        <img src="/vite.svg" class="w-12 h-12 pt-2 drop-shadow-md hover:drop-shadow-xl" alt="Vite logo" />
      </a>
      <a href="https://tauri.app" target="_blank">
        <img src="/tauri.svg" class="logo tauri" alt="Tauri logo" />
      </a>
      <a href="https://vuejs.org/" target="_blank">
        <img src="./assets/vue.svg" class="logo vue" alt="Vue logo" />
      </a>
    </div>

    <p>Click on the Tauri, Vite, and Vue logos to learn more.</p>

    <p>
      Recommended IDE setup:
      <a href="https://code.visualstudio.com/" target="_blank">VS Code</a>
      +
      <a href="https://github.com/johnsoncodehk/volar" target="_blank">Volar</a>
      +
      <a href="https://github.com/tauri-apps/tauri-vscode" target="_blank"
        >Tauri</a
      >
      +
      <a href="https://github.com/rust-lang/rust-analyzer" target="_blank"
        >rust-analyzer</a
      >
    </p>

    <Greet />
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
