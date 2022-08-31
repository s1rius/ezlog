<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/tauri";

let ologs: any[] = []

async function fetchLogs() {
            await invoke('read_log')
                .then((logs: any) => {
                    ologs = logs;
                })
                .catch((error: any) => {
                    console.error('Error fetching logs:', error);
                });
        }
</script>

<template>
    <div>
        <h1>Log List</h1>
        <ul>
            <li v-for="log in ologs" :key="log">{{ log }}</li>
        </ul>
    </div>
</template>
  
  