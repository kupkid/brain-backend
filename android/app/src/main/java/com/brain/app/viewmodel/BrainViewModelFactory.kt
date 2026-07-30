package com.brain.app.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import com.brain.app.data.AgentRepository
import com.brain.app.data.BrainSettings

class BrainViewModelFactory(
    private val application: Application,
    private val settings: BrainSettings
) : ViewModelProvider.Factory {
    @Suppress("UNCHECKED_CAST")
    override fun <T : ViewModel> create(modelClass: Class<T>): T {
        val repo = AgentRepository(settings)
        return BrainViewModel(application, repo, settings) as T
    }
}
