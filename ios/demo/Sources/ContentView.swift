//
//  ContentView.swift
//  demo
//
//  Created by s1rius on 2022/6/8.
//

import SwiftUI
import EZLog

struct ContentView: View {
    var body: some View {
        NavigationView {
            VStack {
                Button{
                    ezlogSampleInit()
                } label: {
                    Label("INIT", systemImage: "chevron.right.circle")
                        .labelStyle(.titleOnly)
                        .padding()
                }
                
                Button{
                    aLog()
                } label: {
                    Label("LOG", systemImage: "chevron.right.circle")
                        .labelStyle(.titleOnly)
                        .padding()
                }
                
                Button{
                    logs()
                } label: {
                    Label("LOGS", systemImage: "chevron.right.circle")
                        .labelStyle(.titleOnly)
                        .padding()
                }
                
                Button{
                    ezlog_flush_all()
                } label: {
                    Label("FLUSH", systemImage: "chevron.right.circle")
                        .labelStyle(.titleOnly)
                        .padding()
                }
                
                Button{
                    requestLogsByDate(date: Date())
                } label: {
                    Label("GET LOG FILES", systemImage: "chevron.right.circle")
                        .labelStyle(.titleOnly)
                        .padding()
                }
            }.navigationTitle("EZLog")
        }
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}
