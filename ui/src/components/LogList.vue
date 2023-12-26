<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/tauri";

let ologs: any[] = []

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

    await invoke('parse_header_and_extra', {file_path: path}).then((result: any) => {
        console.error("sdfasdf", result);
    }).catch((error: any) => {})
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
  
  