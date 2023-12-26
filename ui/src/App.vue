<script setup lang="ts">
// This starter template is using Vue 3 <script setup> SFCs
// Check out https://vuejs.org/api/sfc-script-setup.html#script-setup
import Greet from "./components/Greet.vue";
import LogList from "./components/LogList.vue";

import { listen } from '@tauri-apps/api/event'

listen('tauri://file-drop', (event) => {
  if (event.payload && event.payload.length > 0) {
      const firstFilePath = event.payload[0];
      console.log('First file dropped:', firstFilePath);
      // Now you can do something with the first file path
      // For example, reading the file content or processing the file
      
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
