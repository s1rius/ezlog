import wtf.s1.buildsrc.*

plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "wtf.s1.ezlog.benchmarkable"
    compileSdk = Versions.compileVersion

    defaultConfig {
        minSdk = Versions.minSdkVersion
        targetSdk = Versions.targetSdkVersion

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
}

dependencies {
    api(project(":lib-ezlog"))
    //api(Deps.ezlogOnline)
    api(Deps.logan)
    api(Deps.xlog)

    androidTestImplementation(Deps.AndroidX.testJunit)
    androidTestImplementation(Deps.AndroidX.espresso)
}